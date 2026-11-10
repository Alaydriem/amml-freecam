use freecam_core::structs::BoundedQueue;

#[test]
fn push_past_capacity_evicts_oldest() {
    let mut queue = BoundedQueue::new(3);
    for item in 1..=5 {
        queue.push(item);
        assert!(queue.len() <= 3);
    }

    let popped: Vec<i32> = std::iter::from_fn(|| queue.pop()).collect();
    assert_eq!(popped, [3, 4, 5]);
    assert_eq!(queue.dropped(), 2);
    assert!(queue.is_empty());
}

#[test]
fn capacity_is_fixed() {
    let mut queue = BoundedQueue::new(8);
    for item in 0..1000 {
        queue.push(item);
    }

    assert_eq!(queue.capacity(), 8);
    assert_eq!(queue.len(), 8);
    assert_eq!(queue.dropped(), 992);
}

#[test]
fn zero_capacity_keeps_nothing() {
    let mut queue = BoundedQueue::new(0);
    queue.push(1);

    assert!(queue.is_empty());
    assert_eq!(queue.dropped(), 1);
}
