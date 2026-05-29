use ideot::marks::SessionMarks;

#[test]
fn stores_current_file_in_first_available_mark_slot() {
    let mut marks = SessionMarks::default();

    let slot = marks.mark("src/main.rs");

    assert_eq!(slot, 1);
    assert_eq!(marks.get(1).map(String::as_str), Some("src/main.rs"));
}

#[test]
fn remarking_existing_file_keeps_single_entry() {
    let mut marks = SessionMarks::default();

    marks.mark("src/main.rs");
    let slot = marks.mark("src/main.rs");

    assert_eq!(slot, 1);
    assert_eq!(marks.iter().count(), 1);
}
