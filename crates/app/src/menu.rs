//! Native macOS Menu Bar
//!
//! This module creates and manages the native macOS menu bar using the cocoa crate.
//! The menu bar follows standard macOS conventions and integrates with the app's
//! keyboard shortcuts.

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::appkit::{
    NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem,
};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::{id, nil, selector};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::foundation::{NSAutoreleasePool, NSString};
#[cfg(target_os = "macos")]
use objc::runtime::Sel;

/// Menu action identifiers for routing menu selections to app handlers.
/// This enum will be used in the future to handle custom menu actions
/// that need to be routed to the Rust application code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MenuAction {
    // File menu
    Open,
    Close,
    Save,
    SaveAs,
    ExportPdf,
    ExportImages,

    // Edit menu
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Find,

    // View menu
    ZoomIn,
    ZoomOut,
    ActualSize,
    FitPage,
    FitWidth,
    ShowThumbnails,
    ShowAnnotations,

    // Go menu
    NextPage,
    PreviousPage,
    FirstPage,
    LastPage,
    GoToPage,

    // Window menu
    Minimize,
    Zoom,

    // Help menu
    About,
}

/// Sets up the native macOS menu bar for the application.
///
/// This function should be called once at application startup, after
/// the NSApplication has been initialized but before the event loop starts.
///
/// # Safety
/// This function uses unsafe Objective-C calls to create native menus.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn setup_menu_bar() {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Create the main menu bar
        let main_menu = NSMenu::new(nil).autorelease();

        // Create and add each menu
        add_app_menu(main_menu);
        add_file_menu(main_menu);
        add_edit_menu(main_menu);
        add_view_menu(main_menu);
        add_go_menu(main_menu);
        add_window_menu(main_menu);
        add_help_menu(main_menu);

        // Set as the main menu
        let app = NSApp();
        app.setMainMenu_(main_menu);
    }
}

/// No-op on non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn setup_menu_bar() {
    // No native menu bar on other platforms
}

// Helper function to create an NSString from a &str
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn ns_string(s: &str) -> id {
    NSString::alloc(nil).init_str(s)
}

// Helper function to create a menu item with a key equivalent
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn menu_item(title: &str, action: Sel, key: &str, modifiers: NSEventModifierFlags) -> id {
    let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
        ns_string(title),
        action,
        ns_string(key),
    );
    item.setKeyEquivalentModifierMask_(modifiers);
    item.autorelease()
}

// Helper function to create a menu item without key equivalent
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn menu_item_no_key(title: &str, action: Sel) -> id {
    NSMenuItem::alloc(nil)
        .initWithTitle_action_keyEquivalent_(ns_string(title), action, ns_string(""))
        .autorelease()
}

// Helper function to create a menu item without action (for placeholders)
// Menu items with no action selector are automatically disabled by macOS
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn menu_item_disabled(title: &str) -> id {
    // Creating a menu item with a null selector causes it to be automatically
    // disabled by macOS (greyed out and non-clickable)
    NSMenuItem::alloc(nil)
        .initWithTitle_action_keyEquivalent_(ns_string(title), Sel::from_ptr(std::ptr::null()), ns_string(""))
        .autorelease()
}

// Helper function to create a separator item
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn separator_item() -> id {
    NSMenuItem::separatorItem(nil)
}


/// Add the application menu (PDF Editor menu with About, Preferences, Quit)
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_app_menu(main_menu: id) {
    let app_menu = NSMenu::new(nil).autorelease();

    // About PDF Editor
    app_menu.addItem_(menu_item_no_key(
        "About PDF Editor",
        selector("orderFrontStandardAboutPanel:"),
    ));

    app_menu.addItem_(separator_item());

    // Hide PDF Editor (Cmd+H)
    app_menu.addItem_(menu_item(
        "Hide PDF Editor",
        selector("hide:"),
        "h",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Hide Others (Cmd+Option+H)
    app_menu.addItem_(menu_item(
        "Hide Others",
        selector("hideOtherApplications:"),
        "h",
        NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSAlternateKeyMask,
    ));

    // Show All
    app_menu.addItem_(menu_item_no_key("Show All", selector("unhideAllApplications:")));

    app_menu.addItem_(separator_item());

    // Quit PDF Editor (Cmd+Q)
    app_menu.addItem_(menu_item(
        "Quit PDF Editor",
        selector("terminate:"),
        "q",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Create menu bar item for app menu
    let app_menu_item = NSMenuItem::new(nil).autorelease();
    app_menu_item.setSubmenu_(app_menu);
    main_menu.addItem_(app_menu_item);
}

/// Add the File menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_file_menu(main_menu: id) {
    let file_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("File")).autorelease();

    // Open... (Cmd+O)
    // Note: We use openDocument: which is the standard action for File > Open
    // The winit event loop will handle the actual file opening through keyboard events
    file_menu.addItem_(menu_item(
        "Open...",
        selector("openDocument:"),
        "o",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Open Recent submenu (placeholder - will be populated dynamically)
    let recent_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("Open Recent")).autorelease();
    let recent_item = NSMenuItem::alloc(nil)
        .initWithTitle_action_keyEquivalent_(ns_string("Open Recent"), Sel::from_ptr(std::ptr::null()), ns_string(""))
        .autorelease();
    recent_item.setSubmenu_(recent_menu);
    file_menu.addItem_(recent_item);

    file_menu.addItem_(separator_item());

    // Close (Cmd+W)
    file_menu.addItem_(menu_item(
        "Close",
        selector("performClose:"),
        "w",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Save (Cmd+S) - disabled until we have save functionality
    let save_item = menu_item(
        "Save",
        selector("saveDocument:"),
        "s",
        NSEventModifierFlags::NSCommandKeyMask,
    );
    file_menu.addItem_(save_item);

    // Save As... (Cmd+Shift+S)
    let save_as_item = menu_item(
        "Save As...",
        selector("saveDocumentAs:"),
        "s",
        NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSShiftKeyMask,
    );
    file_menu.addItem_(save_as_item);

    file_menu.addItem_(separator_item());

    // Export as PDF... (placeholder, disabled)
    file_menu.addItem_(menu_item_disabled("Export as PDF..."));

    // Export as Images... (placeholder, disabled)
    file_menu.addItem_(menu_item_disabled("Export as Images..."));

    // Create menu bar item
    let file_menu_item = NSMenuItem::new(nil).autorelease();
    file_menu_item.setSubmenu_(file_menu);
    main_menu.addItem_(file_menu_item);
}

/// Add the Edit menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_edit_menu(main_menu: id) {
    let edit_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("Edit")).autorelease();

    // Undo (Cmd+Z)
    edit_menu.addItem_(menu_item(
        "Undo",
        selector("undo:"),
        "z",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Redo (Cmd+Shift+Z)
    edit_menu.addItem_(menu_item(
        "Redo",
        selector("redo:"),
        "z",
        NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSShiftKeyMask,
    ));

    edit_menu.addItem_(separator_item());

    // Cut (Cmd+X)
    edit_menu.addItem_(menu_item(
        "Cut",
        selector("cut:"),
        "x",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Copy (Cmd+C)
    edit_menu.addItem_(menu_item(
        "Copy",
        selector("copy:"),
        "c",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Paste (Cmd+V)
    edit_menu.addItem_(menu_item(
        "Paste",
        selector("paste:"),
        "v",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Select All (Cmd+A)
    edit_menu.addItem_(menu_item(
        "Select All",
        selector("selectAll:"),
        "a",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    edit_menu.addItem_(separator_item());

    // Find... (Cmd+F)
    // This will be handled by the app's search functionality
    edit_menu.addItem_(menu_item(
        "Find...",
        selector("performFindPanelAction:"),
        "f",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Create menu bar item
    let edit_menu_item = NSMenuItem::new(nil).autorelease();
    edit_menu_item.setSubmenu_(edit_menu);
    main_menu.addItem_(edit_menu_item);
}

/// Add the View menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_view_menu(main_menu: id) {
    let view_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("View")).autorelease();

    // Zoom In (Cmd+=)
    view_menu.addItem_(menu_item(
        "Zoom In",
        selector("zoomIn:"),
        "=",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Zoom Out (Cmd+-)
    view_menu.addItem_(menu_item(
        "Zoom Out",
        selector("zoomOut:"),
        "-",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Actual Size (Cmd+0)
    view_menu.addItem_(menu_item(
        "Actual Size",
        selector("zoomToActualSize:"),
        "0",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    view_menu.addItem_(separator_item());

    // Fit Page (no standard shortcut)
    view_menu.addItem_(menu_item_no_key("Fit Page", selector("zoomToFit:")));

    // Fit Width (no standard shortcut)
    view_menu.addItem_(menu_item_no_key("Fit Width", selector("zoomToWidth:")));

    view_menu.addItem_(separator_item());

    // Show Thumbnails (Cmd+T - note: conflicts with some apps, might change)
    view_menu.addItem_(menu_item(
        "Show Thumbnails",
        selector("toggleThumbnails:"),
        "t",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Show Annotations
    view_menu.addItem_(menu_item_no_key("Show Annotations", selector("toggleAnnotations:")));

    // Create menu bar item
    let view_menu_item = NSMenuItem::new(nil).autorelease();
    view_menu_item.setSubmenu_(view_menu);
    main_menu.addItem_(view_menu_item);
}

/// Add the Go menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_go_menu(main_menu: id) {
    let go_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("Go")).autorelease();

    // Next Page (Right Arrow / Page Down)
    // Note: Key equivalents for arrow keys need special handling
    go_menu.addItem_(menu_item_no_key("Next Page", selector("goToNextPage:")));

    // Previous Page (Left Arrow / Page Up)
    go_menu.addItem_(menu_item_no_key("Previous Page", selector("goToPreviousPage:")));

    go_menu.addItem_(separator_item());

    // First Page (Home)
    go_menu.addItem_(menu_item_no_key("First Page", selector("goToFirstPage:")));

    // Last Page (End)
    go_menu.addItem_(menu_item_no_key("Last Page", selector("goToLastPage:")));

    go_menu.addItem_(separator_item());

    // Go to Page... (Cmd+G)
    go_menu.addItem_(menu_item(
        "Go to Page...",
        selector("goToPage:"),
        "g",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Create menu bar item
    let go_menu_item = NSMenuItem::new(nil).autorelease();
    go_menu_item.setSubmenu_(go_menu);
    main_menu.addItem_(go_menu_item);
}

/// Add the Window menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_window_menu(main_menu: id) {
    let window_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("Window")).autorelease();

    // Minimize (Cmd+M)
    window_menu.addItem_(menu_item(
        "Minimize",
        selector("performMiniaturize:"),
        "m",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Zoom (no shortcut - this is the macOS window zoom, not PDF zoom)
    window_menu.addItem_(menu_item_no_key("Zoom", selector("performZoom:")));

    window_menu.addItem_(separator_item());

    // Bring All to Front
    window_menu.addItem_(menu_item_no_key(
        "Bring All to Front",
        selector("arrangeInFront:"),
    ));

    // Create menu bar item
    let window_menu_item = NSMenuItem::new(nil).autorelease();
    window_menu_item.setSubmenu_(window_menu);
    main_menu.addItem_(window_menu_item);

    // Tell NSApp this is the Window menu (enables automatic window list)
    let app = NSApp();
    app.setWindowsMenu_(window_menu);
}

/// Add the Help menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_help_menu(main_menu: id) {
    let help_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("Help")).autorelease();

    // PDF Editor Help (Cmd+?)
    // For now this just shows the about panel
    help_menu.addItem_(menu_item(
        "PDF Editor Help",
        selector("showHelp:"),
        "?",
        NSEventModifierFlags::NSCommandKeyMask,
    ));

    // Create menu bar item
    let help_menu_item = NSMenuItem::new(nil).autorelease();
    help_menu_item.setSubmenu_(help_menu);
    main_menu.addItem_(help_menu_item);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_action_enum() {
        // Test that menu actions can be compared
        assert_eq!(MenuAction::Open, MenuAction::Open);
        assert_ne!(MenuAction::Open, MenuAction::Close);
    }

    #[test]
    fn test_menu_action_debug() {
        // Test that menu actions can be debugged
        let action = MenuAction::ZoomIn;
        let debug_str = format!("{:?}", action);
        assert_eq!(debug_str, "ZoomIn");
    }

    #[test]
    fn test_menu_action_clone() {
        // Test that menu actions can be cloned
        let action = MenuAction::Save;
        let cloned = action;
        assert_eq!(action, cloned);
    }

    // Note: Actual menu creation tests require a running macOS application
    // and cannot be easily unit tested. Integration tests should verify
    // menu functionality through the GUI.

    #[test]
    #[cfg(target_os = "macos")]
    fn test_setup_menu_bar_compiles() {
        // This test just verifies the function compiles correctly
        // Actual execution requires a running NSApplication
        // setup_menu_bar() would panic without proper app initialization
        assert!(true);
    }
}
