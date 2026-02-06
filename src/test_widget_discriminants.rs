// Test file to determine Widget discriminant values
// This will help identify which widget type is causing the panic

use crate::ui::widgets::Widget;

#[test]
fn test_widget_discriminants() {
    // Create instances of each widget type to check their discriminants
    // Note: This is a conceptual test - we need to know the actual widget struct constructors

    let widgets = vec![
        // Widget::Button(...), // Need actual Button constructor
        // Widget::Caption(...),
        // etc.
    ];

    for widget in widgets {
        let discriminant = std::mem::discriminant(&widget);
        println!("Widget discriminant: {:?}", discriminant);
    }
}
