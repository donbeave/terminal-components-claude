//! Documentation examples coverage test.
//!
//! Preserves doc-test assertion coverage under `cargo nextest`.

#[test]
fn test_ui_text_slice_cells_doc_example() {
    use junie_tui::ui::text::slice_cells;
    assert_eq!(slice_cells("ab日本cd", 3, 5), "…本cd");
    assert_eq!(slice_cells("日本語", 0, 5), "日本…");
}
