use std::collections::HashMap;

use crate::scheduling_ffi::{
    LabelSelector, LabelSelectorOperator, NodeResourceView, ResourceRequest, SchedulingDecision,
    SchedulingRequest,
};

#[derive(Debug, Default)]
pub struct ClusterResourceScheduler {
    nodes: HashMap<i64, crate::scheduling_ffi::NodeResources>,
    fairness_cursor: usize,
}

impl ClusterResourceScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_nodes(&mut self, updates: Vec<NodeResourceView>) {
        for update in updates {
            self.nodes.insert(update.node_id, update.resources);
        }
        if self.fairness_cursor >= self.nodes.len() {
            self.fairness_cursor = 0;
        }
    }

    pub fn allocate(&mut self, request: &SchedulingRequest) -> SchedulingDecision {
        if self.nodes.is_empty() {
            return SchedulingDecision {
                request_id: request.request_id,
                selected_node_id: -1,
                is_feasible: false,
                is_spillback: false,
            };
        }

        let ordered_nodes = self.ordered_node_ids();
        let start = self
            .preferred_start_index(request.preferred_node_id, &ordered_nodes)
            .unwrap_or(self.fairness_cursor % ordered_nodes.len());

        let mut feasible_unavailable_node = None;
        for offset in 0..ordered_nodes.len() {
            let idx = (start + offset) % ordered_nodes.len();
            let node_id = ordered_nodes[idx];
            let node = self
                .nodes
                .get(&node_id)
                .expect("node id collected from map keys must exist");

            if !matches_label_selector(&node.labels, &request.resource_request.label_selector)
                || node.is_draining
            {
                continue;
            }

            if has_available_resources(node, &request.resource_request) {
                let mutable_node = self
                    .nodes
                    .get_mut(&node_id)
                    .expect("node id collected from map keys must exist");
                subtract_resources(mutable_node, &request.resource_request.resources);
                self.fairness_cursor = (idx + 1) % ordered_nodes.len();
                return SchedulingDecision {
                    request_id: request.request_id,
                    selected_node_id: node_id,
                    is_feasible: true,
                    is_spillback: false,
                };
            }

            if feasible_unavailable_node.is_none()
                && has_total_capacity(node, &request.resource_request)
            {
                feasible_unavailable_node = Some(node_id);
            }
        }

        SchedulingDecision {
            request_id: request.request_id,
            selected_node_id: feasible_unavailable_node.unwrap_or(-1),
            is_feasible: feasible_unavailable_node.is_some(),
            is_spillback: feasible_unavailable_node.is_some(),
        }
    }

    pub fn release(&mut self, node_id: i64, resources: &HashMap<String, f64>) -> bool {
        let Some(node) = self.nodes.get_mut(&node_id) else {
            return false;
        };

        for (name, quantity) in resources {
            let current = *node.available.get(name).unwrap_or(&0.0);
            let total = *node.total.get(name).unwrap_or(&current);
            node.available
                .insert(name.clone(), (current + *quantity).min(total));
        }
        true
    }

    fn ordered_node_ids(&self) -> Vec<i64> {
        let mut ordered = self.nodes.keys().copied().collect::<Vec<_>>();
        ordered.sort_unstable();
        ordered
    }

    fn preferred_start_index(&self, preferred_node_id: i64, ordered_nodes: &[i64]) -> Option<usize> {
        if preferred_node_id < 0 {
            return None;
        }
        ordered_nodes.iter().position(|node_id| *node_id == preferred_node_id)
    }
}

fn matches_label_selector(labels: &HashMap<String, String>, selector: &LabelSelector) -> bool {
    selector.constraints.iter().all(|constraint| {
        let value = labels.get(&constraint.key);
        match constraint.op {
            LabelSelectorOperator::Unspecified => true,
            LabelSelectorOperator::In => value
                .map(|label| constraint.values.iter().any(|candidate| candidate == label))
                .unwrap_or(false),
            LabelSelectorOperator::NotIn => value
                .map(|label| constraint.values.iter().all(|candidate| candidate != label))
                .unwrap_or(true),
        }
    })
}

fn has_available_resources(
    node: &crate::scheduling_ffi::NodeResources,
    request: &ResourceRequest,
) -> bool {
    if request.requires_object_store_memory {
        let object_store = *node.available.get("object_store_memory").unwrap_or(&0.0);
        if object_store <= 0.0 {
            return false;
        }
    }

    request.resources.iter().all(|(resource, amount)| {
        let available = *node.available.get(resource).unwrap_or(&0.0);
        available + f64::EPSILON >= *amount
    })
}

fn has_total_capacity(node: &crate::scheduling_ffi::NodeResources, request: &ResourceRequest) -> bool {
    request.resources.iter().all(|(resource, amount)| {
        let total = *node.total.get(resource).unwrap_or(&0.0);
        total + f64::EPSILON >= *amount
    })
}

fn subtract_resources(node: &mut crate::scheduling_ffi::NodeResources, resources: &HashMap<String, f64>) {
    for (name, quantity) in resources {
        let available = *node.available.get(name).unwrap_or(&0.0);
        node.available
            .insert(name.clone(), (available - *quantity).max(0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduling_ffi::{
        LabelConstraint, LabelSelectorOperator, NodeResources, ResourceRequest,
    };

    fn node_resources(cpu_total: f64, cpu_available: f64, labels: &[(&str, &str)]) -> NodeResources {
        let mut total = HashMap::new();
        total.insert("CPU".to_string(), cpu_total);
        let mut available = HashMap::new();
        available.insert("CPU".to_string(), cpu_available);
        let mut node_labels = HashMap::new();
        for (key, value) in labels {
            node_labels.insert((*key).to_string(), (*value).to_string());
        }

        NodeResources {
            total,
            available,
            load: HashMap::new(),
            normal_task_resources: HashMap::new(),
            labels: node_labels,
            idle_resource_duration_ms: 0,
            is_draining: false,
            draining_deadline_timestamp_ms: 0,
            last_resource_update_ms: 0,
            latest_resources_normal_task_timestamp: 0,
            object_pulls_queued: false,
        }
    }

    fn request(
        request_id: i64,
        preferred_node_id: i64,
        cpu: f64,
        selector: LabelSelector,
    ) -> SchedulingRequest {
        let mut resources = HashMap::new();
        resources.insert("CPU".to_string(), cpu);
        SchedulingRequest {
            request_id,
            preferred_node_id,
            resource_request: ResourceRequest {
                resources,
                requires_object_store_memory: false,
                label_selector: selector,
            },
        }
    }

    #[test]
    fn round_robin_fairness_rotates_nodes() {
        let mut scheduler = ClusterResourceScheduler::new();
        scheduler.update_nodes(vec![
            NodeResourceView {
                node_id: 1,
                resources: node_resources(2.0, 2.0, &[]),
            },
            NodeResourceView {
                node_id: 2,
                resources: node_resources(2.0, 2.0, &[]),
            },
        ]);

        let selector = LabelSelector { constraints: vec![] };
        let first = scheduler.allocate(&request(1, -1, 1.0, selector.clone()));
        let second = scheduler.allocate(&request(2, -1, 1.0, selector));

        assert_eq!(first.selected_node_id, 1);
        assert_eq!(second.selected_node_id, 2);
    }

    #[test]
    fn preferred_node_is_chosen_when_schedulable() {
        let mut scheduler = ClusterResourceScheduler::new();
        scheduler.update_nodes(vec![
            NodeResourceView {
                node_id: 10,
                resources: node_resources(4.0, 4.0, &[]),
            },
            NodeResourceView {
                node_id: 20,
                resources: node_resources(4.0, 4.0, &[]),
            },
        ]);

        let decision = scheduler.allocate(&request(99, 20, 2.0, LabelSelector { constraints: vec![] }));
        assert_eq!(decision.selected_node_id, 20);
        assert!(decision.is_feasible);
        assert!(!decision.is_spillback);
    }

    #[test]
    fn label_selector_filters_nodes() {
        let mut scheduler = ClusterResourceScheduler::new();
        scheduler.update_nodes(vec![
            NodeResourceView {
                node_id: 1,
                resources: node_resources(4.0, 4.0, &[("zone", "a")]),
            },
            NodeResourceView {
                node_id: 2,
                resources: node_resources(4.0, 4.0, &[("zone", "b")]),
            },
        ]);

        let selector = LabelSelector {
            constraints: vec![LabelConstraint {
                key: "zone".to_string(),
                op: LabelSelectorOperator::In,
                values: vec!["b".to_string()],
            }],
        };
        let decision = scheduler.allocate(&request(3, -1, 1.0, selector));
        assert_eq!(decision.selected_node_id, 2);
    }

    #[test]
    fn spillback_when_total_capacity_exists_but_available_is_exhausted() {
        let mut scheduler = ClusterResourceScheduler::new();
        scheduler.update_nodes(vec![NodeResourceView {
            node_id: 1,
            resources: node_resources(2.0, 0.0, &[]),
        }]);

        let decision = scheduler.allocate(&request(4, -1, 1.0, LabelSelector { constraints: vec![] }));
        assert_eq!(decision.selected_node_id, 1);
        assert!(decision.is_feasible);
        assert!(decision.is_spillback);
    }

    #[test]
    fn release_restores_capacity_up_to_total() {
        let mut scheduler = ClusterResourceScheduler::new();
        scheduler.update_nodes(vec![NodeResourceView {
            node_id: 7,
            resources: node_resources(2.0, 2.0, &[]),
        }]);

        let decision = scheduler.allocate(&request(5, -1, 2.0, LabelSelector { constraints: vec![] }));
        assert_eq!(decision.selected_node_id, 7);

        let mut released = HashMap::new();
        released.insert("CPU".to_string(), 1.5);
        assert!(scheduler.release(7, &released));

        let decision = scheduler.allocate(&request(6, -1, 1.0, LabelSelector { constraints: vec![] }));
        assert_eq!(decision.selected_node_id, 7);
        assert!(decision.is_feasible);
    }
}
