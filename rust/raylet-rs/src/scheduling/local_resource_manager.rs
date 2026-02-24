use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::scheduling_ffi::{NodeResources, ResourceRequest};

const UNTRACKED_IDLE_MARKER: i64 = i64::MIN;

type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkFootprint {
    NodeWorkers,
    PullingTaskArguments,
}

#[derive(Debug, Clone)]
struct IdleTimeState {
    current: Option<i64>,
    saved: Option<i64>,
}

#[derive(Debug, Clone, Eq)]
enum WorkArtifact {
    Footprint(WorkFootprint),
    Resource(String),
}

impl PartialEq for WorkArtifact {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Footprint(lhs), Self::Footprint(rhs)) => lhs == rhs,
            (Self::Resource(lhs), Self::Resource(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl Hash for WorkArtifact {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Footprint(item) => {
                0u8.hash(state);
                item.hash(state);
            }
            Self::Resource(resource_name) => {
                1u8.hash(state);
                resource_name.hash(state);
            }
        }
    }
}

pub struct LocalResourceManager {
    total: HashMap<String, f64>,
    available: HashMap<String, f64>,
    idle_time_states: HashMap<WorkArtifact, IdleTimeState>,
    clock: Clock,
}

impl LocalResourceManager {
    pub fn new(node_resources: NodeResources) -> Self {
        Self::new_with_clock(node_resources, Arc::new(now_ms))
    }

    pub fn new_with_clock(node_resources: NodeResources, clock: Clock) -> Self {
        let now = clock();
        let mut idle_time_states = HashMap::with_capacity(node_resources.total.len());
        for resource_name in node_resources.total.keys() {
            idle_time_states.insert(
                WorkArtifact::Resource(resource_name.clone()),
                IdleTimeState {
                    current: Some(now),
                    saved: None,
                },
            );
        }

        Self {
            total: node_resources.total,
            available: node_resources.available,
            idle_time_states,
            clock,
        }
    }

    pub fn get_available(&self, resource_name: &str) -> Option<f64> {
        self.available.get(resource_name).copied()
    }

    pub fn is_available_resource_empty(&self, resource_name: &str) -> bool {
        self.get_available(resource_name).unwrap_or_default() <= 0.0
    }

    pub fn allocate(&mut self, request: &ResourceRequest) -> bool {
        for (resource_name, required) in &request.resources {
            let available = self.available.get(resource_name).copied().unwrap_or_default();
            if available + f64::EPSILON < *required {
                return false;
            }
        }

        for (resource_name, required) in &request.resources {
            if *required <= 0.0 {
                continue;
            }
            if let Some(available) = self.available.get_mut(resource_name) {
                *available -= *required;
                self.set_resource_non_idle(resource_name);
            }
        }

        true
    }

    pub fn release(&mut self, released: &HashMap<String, f64>) {
        for (resource_name, amount) in released {
            if *amount <= 0.0 {
                continue;
            }
            let Some(total) = self.total.get(resource_name).copied() else {
                continue;
            };
            let Some(available) = self.available.get_mut(resource_name) else {
                continue;
            };
            *available = (*available + *amount).min(total);
            if (*available - total).abs() <= f64::EPSILON {
                self.set_resource_idle(resource_name);
            }
        }
    }

    pub fn add_resource_instances(&mut self, resource_name: &str, amount: f64) {
        if amount <= 0.0 {
            return;
        }
        let Some(total) = self.total.get(resource_name).copied() else {
            return;
        };
        let Some(available) = self.available.get_mut(resource_name) else {
            return;
        };
        *available = (*available + amount).min(total);
        if (*available - total).abs() <= f64::EPSILON {
            self.set_resource_idle(resource_name);
        }
    }

    pub fn subtract_resource_instances(
        &mut self,
        resource_name: &str,
        amount: f64,
        allow_going_negative: bool,
    ) -> f64 {
        if amount <= 0.0 {
            return 0.0;
        }

        let available = self.available.entry(resource_name.to_string()).or_insert(0.0);
        let underflow = if allow_going_negative {
            0.0
        } else {
            (amount - *available).max(0.0)
        };

        *available -= amount;
        if !allow_going_negative {
            *available = (*available).max(0.0);
        }

        self.set_resource_non_idle(resource_name);
        underflow
    }

    pub fn mark_footprint_as_busy(&mut self, item: WorkFootprint) {
        if let Some(prev) = self.idle_time_states.get(&WorkArtifact::Footprint(item)) {
            if prev.current.is_none() {
                return;
            }
        }

        self.idle_time_states.insert(
            WorkArtifact::Footprint(item),
            IdleTimeState {
                current: None,
                saved: None,
            },
        );

        for (artifact, idle_state) in &mut self.idle_time_states {
            if matches!(artifact, WorkArtifact::Footprint(_)) {
                idle_state.saved = None;
            }
        }
    }

    pub fn maybe_mark_footprint_as_busy(&mut self, item: WorkFootprint) {
        let key = WorkArtifact::Footprint(item);
        if let Some(state) = self.idle_time_states.get(&key) {
            if state.current.is_none() {
                return;
            }
        }

        if let Some(state) = self.idle_time_states.get_mut(&key) {
            state.saved = state.current;
            state.current = None;
            return;
        }

        self.idle_time_states.insert(
            key,
            IdleTimeState {
                current: None,
                saved: Some(UNTRACKED_IDLE_MARKER),
            },
        );
    }

    pub fn mark_footprint_as_idle(&mut self, item: WorkFootprint) {
        let key = WorkArtifact::Footprint(item);
        if let Some(prev) = self.idle_time_states.get(&key) {
            if prev.current.is_some() && prev.saved.is_none() {
                return;
            }
        }

        let saved_idle_time = self.idle_time_states.get(&key).and_then(|state| state.saved);
        if saved_idle_time == Some(UNTRACKED_IDLE_MARKER) {
            self.idle_time_states.remove(&key);
            return;
        }

        if let Some(saved) = saved_idle_time {
            if let Some(state) = self.idle_time_states.get_mut(&key) {
                state.current = Some(saved);
                state.saved = None;
            }
            return;
        }

        let now = (self.clock)();
        self.idle_time_states
            .entry(key)
            .and_modify(|state| {
                state.current = Some(now);
                state.saved = None;
            })
            .or_insert(IdleTimeState {
                current: Some(now),
                saved: None,
            });
    }

    pub fn get_resource_idle_time(&self) -> Option<i64> {
        let mut all_idle_time = i64::MIN;
        for idle_state in self.idle_time_states.values() {
            let current = idle_state.current?;
            all_idle_time = all_idle_time.max(current);
        }
        Some(all_idle_time)
    }

    pub fn is_local_node_idle(&self) -> bool {
        self.get_resource_idle_time().is_some()
    }

    fn set_resource_non_idle(&mut self, resource_name: &str) {
        let state = self
            .idle_time_states
            .entry(WorkArtifact::Resource(resource_name.to_string()))
            .or_insert(IdleTimeState {
                current: Some((self.clock)()),
                saved: None,
            });
        state.current = None;
    }

    fn set_resource_idle(&mut self, resource_name: &str) {
        let state = self
            .idle_time_states
            .entry(WorkArtifact::Resource(resource_name.to_string()))
            .or_insert(IdleTimeState {
                current: Some((self.clock)()),
                saved: None,
            });
        state.current = Some((self.clock)());
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI64, Ordering};

    use super::*;
    use crate::scheduling_ffi::{LabelSelector, ResourceRequest};

    fn fake_manager() -> (LocalResourceManager, Arc<AtomicI64>) {
        let now = Arc::new(AtomicI64::new(1000));
        let fake_clock = {
            let now = now.clone();
            Arc::new(move || now.load(Ordering::Relaxed))
        };
        let mut total = HashMap::new();
        total.insert("CPU".to_string(), 2.0);
        total.insert("GPU".to_string(), 1.0);
        let mut available = HashMap::new();
        available.insert("CPU".to_string(), 2.0);
        available.insert("GPU".to_string(), 1.0);

        (
            LocalResourceManager::new_with_clock(
                NodeResources {
                    total,
                    available,
                    load: HashMap::new(),
                    normal_task_resources: HashMap::new(),
                    labels: HashMap::new(),
                    idle_resource_duration_ms: 0,
                    is_draining: false,
                    draining_deadline_timestamp_ms: -1,
                    last_resource_update_ms: 0,
                    latest_resources_normal_task_timestamp: 0,
                    object_pulls_queued: false,
                },
                fake_clock,
            ),
            now,
        )
    }

    #[test]
    fn allocate_release_roundtrip() {
        let (mut manager, _) = fake_manager();

        let mut resources = HashMap::new();
        resources.insert("CPU".to_string(), 1.0);
        let request = ResourceRequest {
            resources,
            requires_object_store_memory: false,
            label_selector: LabelSelector {
                constraints: Vec::new(),
            },
        };

        assert!(manager.allocate(&request));
        assert_eq!(manager.get_available("CPU"), Some(1.0));
        assert!(!manager.is_local_node_idle());

        let mut released = HashMap::new();
        released.insert("CPU".to_string(), 1.0);
        manager.release(&released);
        assert_eq!(manager.get_available("CPU"), Some(2.0));
        assert!(manager.is_local_node_idle());
    }

    #[test]
    fn subtract_resource_instances_returns_underflow() {
        let (mut manager, _) = fake_manager();
        let underflow = manager.subtract_resource_instances("CPU", 3.0, false);
        assert_eq!(underflow, 1.0);
        assert_eq!(manager.get_available("CPU"), Some(0.0));
    }

    #[test]
    fn maybe_mark_footprint_as_busy_restores_idle_time() {
        let (mut manager, now) = fake_manager();
        let initial = manager.get_resource_idle_time().expect("initial idle time");

        now.store(1050, Ordering::Relaxed);
        manager.maybe_mark_footprint_as_busy(WorkFootprint::PullingTaskArguments);
        assert!(!manager.is_local_node_idle());

        now.store(1100, Ordering::Relaxed);
        manager.mark_footprint_as_idle(WorkFootprint::PullingTaskArguments);
        assert_eq!(manager.get_resource_idle_time(), Some(initial));
    }

    #[test]
    fn mark_footprint_busy_resets_idle_time() {
        let (mut manager, now) = fake_manager();

        now.store(1050, Ordering::Relaxed);
        manager.mark_footprint_as_busy(WorkFootprint::NodeWorkers);
        assert!(!manager.is_local_node_idle());

        now.store(1100, Ordering::Relaxed);
        manager.mark_footprint_as_idle(WorkFootprint::NodeWorkers);
        assert_eq!(manager.get_resource_idle_time(), Some(1100));
    }

    #[test]
    fn repeated_mark_footprint_idle_is_noop() {
        let (mut manager, now) = fake_manager();
        manager.mark_footprint_as_idle(WorkFootprint::PullingTaskArguments);
        let first_idle = manager.get_resource_idle_time().expect("idle time");

        now.store(1500, Ordering::Relaxed);
        manager.mark_footprint_as_idle(WorkFootprint::PullingTaskArguments);
        assert_eq!(manager.get_resource_idle_time(), Some(first_idle));
    }
}
