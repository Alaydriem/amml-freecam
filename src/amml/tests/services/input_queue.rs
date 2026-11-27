use alaydriem_freecam::services::InputQueue;
use alaydriem_freecam::types::InputItem;
use amm_core::structs::event::KeybindPhase;
use freecam_core::structs::{FreecamAction, PointerButton};

#[test]
fn drain_gives_the_items_in_order_and_empties_the_queue() {
    let queue = InputQueue::new(8);
    queue.push(InputItem::Look { dx: 3.0, dy: -1.0 });
    queue.push(InputItem::Bind(FreecamAction::FlyForward, KeybindPhase::Pressed));
    queue.push(InputItem::Button(PointerButton::Right, true));
    let mut out = Vec::new();

    queue.drain(&mut out);
    let first = out.clone();
    queue.drain(&mut out);

    assert_eq!(
        first,
        vec![
            InputItem::Look { dx: 3.0, dy: -1.0 },
            InputItem::Bind(FreecamAction::FlyForward, KeybindPhase::Pressed),
            InputItem::Button(PointerButton::Right, true),
        ]
    );
    assert!(out.is_empty());
}

#[test]
fn a_full_queue_drops_the_oldest_item_that_is_not_a_bind_and_counts_it() {
    let queue = InputQueue::new(3);
    queue.push(InputItem::Bind(FreecamAction::FlyForward, KeybindPhase::Released));
    queue.push(InputItem::Wheel(1.0));
    queue.push(InputItem::Wheel(2.0));
    queue.push(InputItem::Wheel(3.0));
    let mut out = Vec::new();

    queue.drain(&mut out);

    assert_eq!(
        out,
        vec![
            InputItem::Bind(FreecamAction::FlyForward, KeybindPhase::Released),
            InputItem::Wheel(2.0),
            InputItem::Wheel(3.0)
        ]
    );
    assert_eq!(queue.dropped(), 1);
}

#[test]
fn consecutive_mouse_moves_add_up_in_one_item() {
    let queue = InputQueue::new(2);
    queue.push(InputItem::Bind(FreecamAction::FlyForward, KeybindPhase::Released));
    for _ in 0..1000 {
        queue.push(InputItem::Look { dx: 1.0, dy: -2.0 });
    }
    let mut out = Vec::new();

    queue.drain(&mut out);

    assert_eq!(
        out,
        vec![
            InputItem::Bind(FreecamAction::FlyForward, KeybindPhase::Released),
            InputItem::Look { dx: 1000.0, dy: -2000.0 }
        ]
    );
    assert_eq!(queue.dropped(), 0);
}

#[test]
fn a_queue_full_of_binds_keeps_them_all() {
    let queue = InputQueue::new(2);
    for action in [FreecamAction::FlyForward, FreecamAction::FlyBack, FreecamAction::FlyLeft] {
        queue.push(InputItem::Bind(action, KeybindPhase::Released));
    }
    queue.push(InputItem::Wheel(1.0));
    let mut out = Vec::new();

    queue.drain(&mut out);

    assert_eq!(out.len(), 3, "{out:?}");
    assert!(out.iter().all(|item| matches!(item, InputItem::Bind(..))));
    assert_eq!(queue.dropped(), 1);
}
