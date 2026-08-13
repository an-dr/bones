use super::*;
use crate::logging::RecordingSink;
use std::sync::{Arc, Mutex};

struct CountingEndpoint {
    ticks: Arc<Mutex<Vec<f32>>>,
}

impl crate::bus::Handler for CountingEndpoint {
    fn handle(&mut self, envelope: &Envelope) {
        if let Some(dt) = read_tick_dt(envelope) {
            self.ticks.lock().unwrap().push(dt);
        }
    }
}

#[test]
fn step_delivers_tick_to_a_subscribed_endpoint() {
    let bus = Bus::new();
    let ticks = Arc::new(Mutex::new(Vec::new()));
    let ep = bus.register(
        "level",
        CountingEndpoint {
            ticks: ticks.clone(),
        },
    );
    ep.subscribe(Tick::TOPIC);

    let runner = Runner::new(bus, Logger::default());
    runner.step(0.016);

    assert_eq!(*ticks.lock().unwrap(), vec![0.016]);
}

#[test]
fn run_for_delivers_ticks_in_order() {
    let bus = Bus::new();
    let ticks = Arc::new(Mutex::new(Vec::new()));
    let budget = crate::bus::EndpointBudget::new(crate::bus::BudgetLimits {
        max_inbound: 1,
        max_publishes: 0,
    });
    let ep = bus.register_with_budget(
        "level",
        CountingEndpoint {
            ticks: ticks.clone(),
        },
        budget.clone(),
    );
    ep.subscribe(Tick::TOPIC);

    let runner = Runner::new(bus, Logger::default());
    runner.run_for(3, 0.016);

    assert_eq!(*ticks.lock().unwrap(), vec![0.016, 0.016, 0.016]);
    assert_eq!(budget.get_drop_counters().inbound, 0);
}

#[test]
fn an_endpoint_not_subscribed_to_tick_receives_nothing() {
    let bus = Bus::new();
    let ticks = Arc::new(Mutex::new(Vec::new()));
    let ep = bus.register(
        "ui",
        CountingEndpoint {
            ticks: ticks.clone(),
        },
    );
    ep.subscribe("ui/*");

    let runner = Runner::new(bus, Logger::default());
    runner.step(0.016);

    assert!(ticks.lock().unwrap().is_empty());
}

#[test]
fn each_step_logs_a_debug_line() {
    let sink = RecordingSink::new();
    let runner = Runner::new(Bus::new(), Logger::new(Arc::new(sink.clone())));

    runner.run_for(2, 0.016);

    assert_eq!(sink.records().len(), 2);
}
