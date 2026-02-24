use std::collections::HashMap;

use crate::scheduling_ffi::{
    LabelSelector, LabelSelectorOperator, NodeResourceView, SchedulingDecision, SchedulingRequest,
};

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone)]
struct NodeState {
    view: NodeResourceView,
    last_selected_tick: u64,
}

#[derive(Debug, Clone)]
struct AllocationRecord {
    node_id: i64,
    resources: HashMap<String, f64>,
}

#[derive(Debug, Default)]
pub struct ClusterResourceScheduler {
    nodes: HashMap<i64, NodeState>,
    allocations: HashMap<i64, AllocationRecord>,
    tick: u64,
}

impl ClusterResourceScheduler {
    pub fn update_cluster_view(&mut self, views: &[NodeResourceView]) {
        let mut next_nodes = HashMap::with_capacity(views.len());
        for view in views {
            let last_selected_tick = self
                .nodes
                .get(&view.node_id)
                .map(|node| node.last_selected_tick)
                .unwrap_or(0);
            next_nodes.insert(
                view.node_id,
                NodeState {
                    view: view.clone(),
                    last_selected_tick,
                },
            );
        }
        self.nodes = next_nodes;
        self.allocations
            .retain(|_, allocation| self.nodes.contains_key(&allocation.node_id));
    }

    pub fn allocate(&mut self, request: &SchedulingRequest) -> SchedulingDecision {
        let required = &request.resource_request.resources;
        let mut feasible_nodes = Vec::new();
        let mut available_nodes = Vec::new();

        for (node_id, node_state) in &self.nodes {
            if node_state.view.resources.is_draining {
                continue;
            }
            if !matches_label_selector(
                &node_state.view.resources.labels,
                &request.resource_request.label_selector,
            ) {
                continue;
            }
            if has_capacity(&node_state.view.resources.total, required) {
                feasible_nodes.push(*node_id);
            }
            if has_capacity(&node_state.view.resources.available, required) {
                available_nodes.push(*node_id);
            }
        }

        if available_nodes.is_empty() {
            return if feasible_nodes.is_empty() {
                SchedulingDecision {
                    request_id: request.request_id,
                    selected_node_id: -1,
                    is_feasible: false,
                    is_spillback: false,
                }
            } else {
                SchedulingDecision {
                    request_id: request.request_id,
                    selected_node_id: feasible_nodes[0],
                    is_feasible: true,
                    is_spillback: true,
                }
            };
        }

        let selected_node_id = if request.preferred_node_id != 0
            && available_nodes.contains(&request.preferred_node_id)
        {
            request.preferred_node_id
        } else {
            self.choose_fair_node(&available_nodes)
        };

        if let Some(node_state) = self.nodes.get_mut(&selected_node_id) {
            consume_resources(&mut node_state.view.resources.available, required);
            self.tick = self.tick.saturating_add(1);
            node_state.last_selected_tick = self.tick;
            self.allocations.insert(
                request.request_id,
                AllocationRecord {
                    node_id: selected_node_id,
                    resources: required.clone(),
                },
            );
        }

        SchedulingDecision {
            request_id: request.request_id,
            selected_node_id,
            is_feasible: true,
            is_spillback: request.preferred_node_id != 0
                && request.preferred_node_id != selected_node_id,
        }
    }

    pub fn release(&mut self, request_id: i64) {
        let Some(allocation) = self.allocations.remove(&request_id) else {
            return;
        };
        let Some(node_state) = self.nodes.get_mut(&allocation.node_id) else {
            return;
        };

        for (resource_name, released) in allocation.resources {
            let available = node_state
                .view
                .resources
                .available
                .entry(resource_name.clone())
                .or_insert(0.0);
            *available += released;

            if let Some(total) = node_state.view.resources.total.get(&resource_name) {
                if *available > *total {
                    *available = *total;
                }
            }
        }
    }

    fn choose_fair_node(&self, candidates: &[i64]) -> i64 {
        *candidates
            .iter()
            .min_by_key(|node_id| {
                self.nodes
                    .get(node_id)
                    .map(|state| state.last_selected_tick)
                    .unwrap_or(0)
            })
            .expect("candidates should not be empty")
    }
}

fn has_capacity(available: &HashMap<String, f64>, required: &HashMap<String, f64>) -> bool {
    required.iter().all(|(name, required_amount)| {
        let available_amount = available.get(name).copied().unwrap_or(0.0);
        available_amount + EPSILON >= *required_amount
    })
}

fn consume_resources(available: &mut HashMap<String, f64>, required: &HashMap<String, f64>) {
    for (name, amount) in required {
        let entry = available.entry(name.clone()).or_insert(0.0);
        *entry -= *amount;
        if *entry < 0.0 {
            *entry = 0.0;
        }
    }
}

fn matches_label_selector(
    labels: &HashMap<String, String>,
    selector: &LabelSelector,
) -> bool {
    selector.constraints.iter().all(|constraint| {
        let node_value = labels.get(&constraint.key);
        match constraint.op {
            LabelSelectorOperator::Unspecified => true,
            LabelSelectorOperator::In => node_value
                .map(|value| constraint.values.iter().any(|candidate| candidate == value))
                .unwrap_or(false),
            LabelSelectorOperator::NotIn => node_value
                .map(|value| constraint.values.iter().all(|candidate| candidate != value))
                .unwrap_or(true),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduling_ffi::{
        LabelConstraint, NodeResources, ResourceRequest, SchedulingRequest,
    };

    fn node(node_id: i64, cpu_total: f64, cpu_available: f64, zone: &str) -> NodeResourceView {
        NodeResourceView {
            node_id,
            resources: NodeResources {
                total: HashMap::from([("CPU".to_string(), cpu_total)]),
                available: HashMap::from([("CPU".to_string(), cpu_available)]),
                load: HashMap::new(),
                normal_task_resources: HashMap::new(),
                labels: HashMap::from([("zone".to_string(), zone.to_string())]),
                idle_resource_duration_ms: 0,
                is_draining: false,
                draining_deadline_timestamp_ms: 0,
                last_resource_update_ms: 0,
                latest_resources_normal_task_timestamp: 0,
                object_pulls_queued: false,
            },
        }
    }

    fn request(request_id: i64, preferred_node_id: i64, cpu: f64) -> SchedulingRequest {
        SchedulingRequest {
            request_id,
            preferred_node_id,
            resource_request: ResourceRequest {
                resources: HashMap::from([("CPU".to_string(), cpu)]),
                requires_object_store_memory: false,
                label_selector: LabelSelector {
                    constraints: vec![],
                },
            },
        }
    }

    #[test]
    fn scheduling_with_preferred_node_uses_preference() {
        let mut scheduler = ClusterResourceScheduler::default();
        scheduler.update_cluster_view(&[node(1, 4.0, 4.0, "a"), node(2, 4.0, 4.0, "b")]);

        let decision = scheduler.allocate(&request(1, 2, 1.0));

        assert!(decision.is_feasible);
        assert_eq!(decision.selected_node_id, 2);
        assert!(!decision.is_spillback);
    }

    #[test]
    fn spread_scheduling_strategy_rotates_nodes() {
        let mut scheduler = ClusterResourceScheduler::default();
        scheduler.update_cluster_view(&[node(10, 4.0, 4.0, "a"), node(11, 4.0, 4.0, "b")]);

        let first = scheduler.allocate(&request(1, 0, 1.0));
        scheduler.release(1);
        let second = scheduler.allocate(&request(2, 0, 1.0));

        assert!(first.is_feasible);
        assert!(second.is_feasible);
        assert_ne!(first.selected_node_id, second.selected_node_id);
    }

    #[test]
    fn label_selector_schedules_matching_node() {
        let mut scheduler = ClusterResourceScheduler::default();
        scheduler.update_cluster_view(&[node(1, 4.0, 4.0, "a"), node(2, 4.0, 4.0, "b")]);

        let mut constrained = request(1, 0, 1.0);
        constrained.resource_request.label_selector.constraints = vec![LabelConstraint {
            key: "zone".to_string(),
            op: LabelSelectorOperator::In,
            values: vec!["b".to_string()],
        }];

        let decision = scheduler.allocate(&constrained);
        assert!(decision.is_feasible);
        assert_eq!(decision.selected_node_id, 2);
    }
}
