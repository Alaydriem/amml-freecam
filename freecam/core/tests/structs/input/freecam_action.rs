use freecam_core::structs::FreecamAction;

#[test]
fn index_past_the_last_action_is_none() {
    assert_eq!(FreecamAction::from_index(11).map(FreecamAction::name), Some("fly_fast"));
    assert_eq!(FreecamAction::from_index(12), None);
}
