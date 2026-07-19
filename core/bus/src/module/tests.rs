use super::*;

#[test]
fn provide_then_consume_round_trips() {
    let mut registry = ServiceRegistry::new();
    registry.provide(42u32).unwrap();
    assert_eq!(registry.consume::<u32>(), Some(42));
}

#[test]
fn consume_without_a_provider_is_none() {
    let mut registry = ServiceRegistry::new();
    assert_eq!(registry.consume::<u32>(), None);
}

#[test]
fn consume_is_single_use() {
    let mut registry = ServiceRegistry::new();
    registry.provide("window".to_string()).unwrap();
    assert_eq!(registry.consume::<String>(), Some("window".to_string()));
    assert_eq!(registry.consume::<String>(), None);
}

#[test]
fn providing_the_same_type_twice_is_an_error() {
    let mut registry = ServiceRegistry::new();
    registry.provide(1u32).unwrap();
    let err = registry.provide(2u32).unwrap_err();
    assert!(err.contains("already provided"), "unexpected error: {err}");
}

#[test]
fn different_types_do_not_collide() {
    let mut registry = ServiceRegistry::new();
    registry.provide(7u32).unwrap();
    registry.provide("seven".to_string()).unwrap();
    assert_eq!(registry.consume::<u32>(), Some(7));
    assert_eq!(registry.consume::<String>(), Some("seven".to_string()));
}

#[test]
fn context_collects_requested_subscriptions() {
    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    ctx.subscribe("gfx/*");
    ctx.subscribe("core/tick");
    assert_eq!(ctx.into_subscriptions(), vec!["gfx/*", "core/tick"]);
}

#[test]
fn context_forwards_service_provide_and_consume() {
    let mut registry = ServiceRegistry::new();
    {
        let mut ctx = ModuleContext::new(&mut registry);
        ctx.provide_service(99u32).unwrap();
    }
    let mut ctx = ModuleContext::new(&mut registry);
    assert_eq!(ctx.consume_service::<u32>(), Some(99));
}
