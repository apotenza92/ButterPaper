//! Native macOS Menu Bar
//!
//! This module creates and manages the native macOS menu bar using the cocoa crate.
//! The menu bar follows standard macOS conventions and integrates with the app's
//! keyboard shortcuts.
//!
//! ## Menu Action Handling
//!
//! Menu actions are handled through a custom Objective-C class `MenuHandler` that
//! receives menu item clicks and sets atomic flags. The main event loop polls these
//! flags to trigger the appropriate Rust code.

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::appkit::{
    NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem,
};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::{id, nil, selector, BOOL, YES};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::foundation::{NSAutoreleasePool, NSString};
#[cfg(target_os = "macos")]
use objc::runtime::{Class, Object, Sel};
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Global flag indicating "Open..." menu item was clicked
static MENU_OPEN_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global flag indicating "Close" menu item was clicked
static MENU_CLOSE_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global flag indicating "Save" menu item was clicked
static MENU_SAVE_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global flag indicating "Save As..." menu item was clicked
static MENU_SAVE_AS_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global flag indicating "Export as PDF..." menu item was clicked
static MENU_EXPORT_PDF_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global flag indicating "Export as Images..." menu item was clicked
static MENU_EXPORT_IMAGES_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global flag indicating "Clear Menu" in Open Recent was clicked
static MENU_CLEAR_RECENT_CLICKED: AtomicBool = AtomicBool::new(false);

/// Global index of which recent file menu item was clicked (0-9, or usize::MAX for none)
static RECENT_FILE_INDEX: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Global storage for recent file paths for menu item lookups
static RECENT_FILE_PATHS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Check if the "Open..." menu action was triggered and reset the flag
pub fn poll_open_action() -> bool {
    MENU_OPEN_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if the "Close" menu action was triggered and reset the flag
pub fn poll_close_action() -> bool {
    MENU_CLOSE_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if the "Save" menu action was triggered and reset the flag
pub fn poll_save_action() -> bool {
    MENU_SAVE_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if the "Save As..." menu action was triggered and reset the flag
pub fn poll_save_as_action() -> bool {
    MENU_SAVE_AS_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if the "Export as PDF..." menu action was triggered and reset the flag
pub fn poll_export_pdf_action() -> bool {
    MENU_EXPORT_PDF_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if the "Export as Images..." menu action was triggered and reset the flag
pub fn poll_export_images_action() -> bool {
    MENU_EXPORT_IMAGES_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if "Clear Menu" was clicked and reset the flag
pub fn poll_clear_recent_action() -> bool {
    MENU_CLEAR_RECENT_CLICKED.swap(false, Ordering::SeqCst)
}

/// Check if a recent file menu item was clicked and return its path
pub fn poll_open_recent_action() -> Option<PathBuf> {
    let index = RECENT_FILE_INDEX.swap(usize::MAX, Ordering::SeqCst);
    if index == usize::MAX {
        return None;
    }

    let paths = RECENT_FILE_PATHS.lock().ok()?;
    paths.get(index).cloned()
}

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

/// Global storage for the MenuHandler instance
#[cfg(target_os = "macos")]
static mut MENU_HANDLER: Option<id> = None;

/// Register the MenuHandler Objective-C class that receives menu actions.
/// This must be called once before creating menu items that use custom selectors.
#[cfg(target_os = "macos")]
unsafe fn register_menu_handler_class() -> *const Class {
    use objc::declare::ClassDecl;
    use std::ffi::CStr;

    // Check if class already exists
    let class_name = c"MenuHandler";
    if let Some(cls) = Class::get(CStr::from_ptr(class_name.as_ptr()).to_str().unwrap()) {
        return cls;
    }

    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("MenuHandler", superclass).unwrap();

    // Add the openFile: method
    extern "C" fn open_file(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_OPEN_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(openFile:),
        open_file as extern "C" fn(&Object, Sel, id),
    );

    // Add the closeWindow: method
    extern "C" fn close_window(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_CLOSE_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(closeWindow:),
        close_window as extern "C" fn(&Object, Sel, id),
    );

    // Add the saveDocument: method
    extern "C" fn save_document(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_SAVE_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(saveDocument:),
        save_document as extern "C" fn(&Object, Sel, id),
    );

    // Add the saveDocumentAs: method
    extern "C" fn save_document_as(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_SAVE_AS_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(saveDocumentAs:),
        save_document_as as extern "C" fn(&Object, Sel, id),
    );

    // Add the exportPdf: method
    extern "C" fn export_pdf(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_EXPORT_PDF_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(exportPdf:),
        export_pdf as extern "C" fn(&Object, Sel, id),
    );

    // Add the exportImages: method
    extern "C" fn export_images(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_EXPORT_IMAGES_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(exportImages:),
        export_images as extern "C" fn(&Object, Sel, id),
    );

    // Add the clearRecentFiles: method
    extern "C" fn clear_recent_files(_this: &Object, _cmd: Sel, _sender: id) {
        MENU_CLEAR_RECENT_CLICKED.store(true, Ordering::SeqCst);
    }
    decl.add_method(
        sel!(clearRecentFiles:),
        clear_recent_files as extern "C" fn(&Object, Sel, id),
    );

    // Add openRecentFile0: through openRecentFile9: methods
    macro_rules! add_recent_file_method {
        ($decl:expr, $index:expr, $sel:ident, $fn_name:ident) => {
            extern "C" fn $fn_name(_this: &Object, _cmd: Sel, _sender: id) {
                RECENT_FILE_INDEX.store($index, Ordering::SeqCst);
            }
            $decl.add_method(
                sel!($sel:),
                $fn_name as extern "C" fn(&Object, Sel, id),
            );
        };
    }

    add_recent_file_method!(decl, 0, openRecentFile0, open_recent_0);
    add_recent_file_method!(decl, 1, openRecentFile1, open_recent_1);
    add_recent_file_method!(decl, 2, openRecentFile2, open_recent_2);
    add_recent_file_method!(decl, 3, openRecentFile3, open_recent_3);
    add_recent_file_method!(decl, 4, openRecentFile4, open_recent_4);
    add_recent_file_method!(decl, 5, openRecentFile5, open_recent_5);
    add_recent_file_method!(decl, 6, openRecentFile6, open_recent_6);
    add_recent_file_method!(decl, 7, openRecentFile7, open_recent_7);
    add_recent_file_method!(decl, 8, openRecentFile8, open_recent_8);
    add_recent_file_method!(decl, 9, openRecentFile9, open_recent_9);

    // Add validateMenuItem: to enable our custom menu items
    extern "C" fn validate_menu_item(_this: &Object, _cmd: Sel, _item: id) -> BOOL {
        YES // Always enable our menu items
    }
    decl.add_method(
        sel!(validateMenuItem:),
        validate_menu_item as extern "C" fn(&Object, Sel, id) -> BOOL,
    );

    decl.register()
}

/// Create an instance of the MenuHandler class
#[cfg(target_os = "macos")]
unsafe fn get_menu_handler() -> id {
    if let Some(handler) = MENU_HANDLER {
        return handler;
    }

    let cls = register_menu_handler_class();
    let handler: id = msg_send![cls, new];
    MENU_HANDLER = Some(handler);
    handler
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

        // Register our custom menu handler class first
        let _handler = get_menu_handler();

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

// Helper function to create a menu item with a custom target
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn menu_item_with_target(
    title: &str,
    action: Sel,
    key: &str,
    modifiers: NSEventModifierFlags,
    target: id,
) -> id {
    let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
        ns_string(title),
        action,
        ns_string(key),
    );
    item.setKeyEquivalentModifierMask_(modifiers);
    let () = msg_send![item, setTarget: target];
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

/// Create the Open Recent submenu populated with recent files
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn create_open_recent_menu(handler: id) -> id {
    let recent_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("Open Recent")).autorelease();

    // Get recent files from the global manager
    let recent_files_arc = crate::recent_files::get_recent_files();
    let recent_files = recent_files_arc.read().ok();

    if let Some(files) = recent_files {
        let paths: Vec<PathBuf> = files.files().to_vec();

        // Store paths in global for lookup when menu items are clicked
        if let Ok(mut stored_paths) = RECENT_FILE_PATHS.lock() {
            *stored_paths = paths.clone();
        }

        // Add menu items for each recent file
        for (i, path) in paths.iter().enumerate() {
            if i >= 10 {
                break; // Max 10 recent files
            }

            // Get the filename for display
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");

            // Select the appropriate selector based on index
            let action_sel = match i {
                0 => sel!(openRecentFile0:),
                1 => sel!(openRecentFile1:),
                2 => sel!(openRecentFile2:),
                3 => sel!(openRecentFile3:),
                4 => sel!(openRecentFile4:),
                5 => sel!(openRecentFile5:),
                6 => sel!(openRecentFile6:),
                7 => sel!(openRecentFile7:),
                8 => sel!(openRecentFile8:),
                9 => sel!(openRecentFile9:),
                _ => continue,
            };

            let item = menu_item_with_target(
                filename,
                action_sel,
                "",
                NSEventModifierFlags::empty(),
                handler,
            );
            recent_menu.addItem_(item);
        }

        // Add separator and "Clear Menu" if there are any recent files
        if !paths.is_empty() {
            recent_menu.addItem_(separator_item());

            let clear_item = menu_item_with_target(
                "Clear Menu",
                sel!(clearRecentFiles:),
                "",
                NSEventModifierFlags::empty(),
                handler,
            );
            recent_menu.addItem_(clear_item);
        }
    }

    // If no recent files, show a disabled "No Recent Items" entry
    let item_count: usize = msg_send![recent_menu, numberOfItems];
    if item_count == 0 {
        recent_menu.addItem_(menu_item_disabled("No Recent Items"));
    }

    recent_menu
}

/// Refresh the Open Recent submenu with current recent files
///
/// This function should be called after loading a file or clearing recent files
/// to update the menu contents.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn refresh_open_recent_menu() {
    unsafe {
        let app = NSApp();
        let main_menu: id = msg_send![app, mainMenu];
        if main_menu == nil {
            return;
        }

        // Find the File menu (index 1, after the app menu)
        let file_menu_item: id = msg_send![main_menu, itemAtIndex: 1i64];
        if file_menu_item == nil {
            return;
        }

        let file_menu: id = msg_send![file_menu_item, submenu];
        if file_menu == nil {
            return;
        }

        // Find the Open Recent submenu item (index 1, after Open...)
        let recent_item: id = msg_send![file_menu, itemAtIndex: 1i64];
        if recent_item == nil {
            return;
        }

        // Create new submenu with updated content
        let handler = get_menu_handler();
        let new_recent_menu = create_open_recent_menu(handler);
        let () = msg_send![recent_item, setSubmenu: new_recent_menu];
    }
}

/// No-op on non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn refresh_open_recent_menu() {}

/// Add the File menu
#[cfg(target_os = "macos")]
#[allow(deprecated)]
unsafe fn add_file_menu(main_menu: id) {
    let file_menu = NSMenu::alloc(nil).initWithTitle_(ns_string("File")).autorelease();

    // Open... (Cmd+O)
    // Uses our custom MenuHandler to set a flag that the event loop polls
    let handler = get_menu_handler();
    file_menu.addItem_(menu_item_with_target(
        "Open...",
        sel!(openFile:),
        "o",
        NSEventModifierFlags::NSCommandKeyMask,
        handler,
    ));

    // Open Recent submenu
    let recent_menu = create_open_recent_menu(handler);
    let recent_item = NSMenuItem::alloc(nil)
        .initWithTitle_action_keyEquivalent_(ns_string("Open Recent"), Sel::from_ptr(std::ptr::null()), ns_string(""))
        .autorelease();
    recent_item.setSubmenu_(recent_menu);
    file_menu.addItem_(recent_item);

    file_menu.addItem_(separator_item());

    // Close (Cmd+W)
    // Uses our custom MenuHandler to set a flag that the event loop polls
    file_menu.addItem_(menu_item_with_target(
        "Close",
        sel!(closeWindow:),
        "w",
        NSEventModifierFlags::NSCommandKeyMask,
        handler,
    ));

    // Save (Cmd+S)
    // Uses our custom MenuHandler to set a flag that the event loop polls
    file_menu.addItem_(menu_item_with_target(
        "Save",
        sel!(saveDocument:),
        "s",
        NSEventModifierFlags::NSCommandKeyMask,
        handler,
    ));

    // Save As... (Cmd+Shift+S)
    // Uses our custom MenuHandler to set a flag that the event loop polls
    file_menu.addItem_(menu_item_with_target(
        "Save As...",
        sel!(saveDocumentAs:),
        "s",
        NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSShiftKeyMask,
        handler,
    ));

    file_menu.addItem_(separator_item());

    // Export as PDF... (Cmd+Shift+E)
    // Uses our custom MenuHandler to set a flag that the event loop polls
    file_menu.addItem_(menu_item_with_target(
        "Export as PDF...",
        sel!(exportPdf:),
        "e",
        NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSShiftKeyMask,
        handler,
    ));

    // Export as Images... (Cmd+Shift+I)
    // Uses our custom MenuHandler to set a flag that the event loop polls
    file_menu.addItem_(menu_item_with_target(
        "Export as Images...",
        sel!(exportImages:),
        "i",
        NSEventModifierFlags::NSCommandKeyMask | NSEventModifierFlags::NSShiftKeyMask,
        handler,
    ));

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

    #[test]
    fn test_poll_open_action_initially_false() {
        // Ensure the flag starts as false (it may have been set by a previous test)
        // Poll it to reset, then check again
        let _ = poll_open_action();
        assert!(!poll_open_action(), "poll_open_action should return false when no action triggered");
    }

    #[test]
    fn test_poll_open_action_resets_after_poll() {
        // Manually set the flag and verify polling resets it
        MENU_OPEN_CLICKED.store(true, Ordering::SeqCst);
        assert!(poll_open_action(), "First poll should return true");
        assert!(!poll_open_action(), "Second poll should return false (flag was reset)");
    }

    #[test]
    fn test_poll_open_action_atomic_swap() {
        // Verify the atomic swap behavior
        MENU_OPEN_CLICKED.store(false, Ordering::SeqCst);
        assert!(!poll_open_action());

        MENU_OPEN_CLICKED.store(true, Ordering::SeqCst);
        // Multiple concurrent reads would all see true until first swap
        let result = poll_open_action();
        assert!(result);
        // After swap, subsequent reads see false
        assert!(!poll_open_action());
    }

    #[test]
    fn test_poll_close_action_initially_false() {
        // Ensure the flag starts as false (it may have been set by a previous test)
        // Poll it to reset, then check again
        let _ = poll_close_action();
        assert!(!poll_close_action(), "poll_close_action should return false when no action triggered");
    }

    #[test]
    fn test_poll_close_action_resets_after_poll() {
        // Manually set the flag and verify polling resets it
        MENU_CLOSE_CLICKED.store(true, Ordering::SeqCst);
        assert!(poll_close_action(), "First poll should return true");
        assert!(!poll_close_action(), "Second poll should return false (flag was reset)");
    }

    #[test]
    fn test_poll_close_action_atomic_swap() {
        // Verify the atomic swap behavior
        MENU_CLOSE_CLICKED.store(false, Ordering::SeqCst);
        assert!(!poll_close_action());

        MENU_CLOSE_CLICKED.store(true, Ordering::SeqCst);
        // Multiple concurrent reads would all see true until first swap
        let result = poll_close_action();
        assert!(result);
        // After swap, subsequent reads see false
        assert!(!poll_close_action());
    }

    #[test]
    fn test_poll_clear_recent_action_initially_false() {
        // Poll to reset, then check again
        let _ = poll_clear_recent_action();
        assert!(!poll_clear_recent_action(), "poll_clear_recent_action should return false when no action triggered");
    }

    #[test]
    fn test_poll_clear_recent_action_resets_after_poll() {
        MENU_CLEAR_RECENT_CLICKED.store(true, Ordering::SeqCst);
        assert!(poll_clear_recent_action(), "First poll should return true");
        assert!(!poll_clear_recent_action(), "Second poll should return false (flag was reset)");
    }

    #[test]
    fn test_poll_open_recent_action_initially_none() {
        // Reset to MAX
        RECENT_FILE_INDEX.store(usize::MAX, Ordering::SeqCst);
        assert!(poll_open_recent_action().is_none(), "poll_open_recent_action should return None when no action triggered");
    }

    #[test]
    fn test_poll_open_recent_action_with_stored_path() {
        // Store a test path
        {
            let mut paths = RECENT_FILE_PATHS.lock().unwrap();
            paths.clear();
            paths.push(PathBuf::from("/test/path.pdf"));
        }

        // Set index to 0
        RECENT_FILE_INDEX.store(0, Ordering::SeqCst);

        let result = poll_open_recent_action();
        assert!(result.is_some(), "Should return Some when index is set");
        assert_eq!(result.unwrap(), PathBuf::from("/test/path.pdf"));

        // Subsequent poll should return None
        assert!(poll_open_recent_action().is_none());
    }

    #[test]
    fn test_poll_open_recent_action_invalid_index() {
        // Store a test path
        {
            let mut paths = RECENT_FILE_PATHS.lock().unwrap();
            paths.clear();
            paths.push(PathBuf::from("/test/path.pdf"));
        }

        // Set index out of bounds
        RECENT_FILE_INDEX.store(10, Ordering::SeqCst);

        let result = poll_open_recent_action();
        assert!(result.is_none(), "Should return None for out-of-bounds index");
    }

    #[test]
    fn test_poll_save_action_initially_false() {
        // Ensure the flag starts as false (it may have been set by a previous test)
        // Poll it to reset, then check again
        let _ = poll_save_action();
        assert!(!poll_save_action(), "poll_save_action should return false when no action triggered");
    }

    #[test]
    fn test_poll_save_action_resets_after_poll() {
        // Manually set the flag and verify polling resets it
        MENU_SAVE_CLICKED.store(true, Ordering::SeqCst);
        assert!(poll_save_action(), "First poll should return true");
        assert!(!poll_save_action(), "Second poll should return false (flag was reset)");
    }

    #[test]
    fn test_poll_save_action_atomic_swap() {
        // Verify the atomic swap behavior
        MENU_SAVE_CLICKED.store(false, Ordering::SeqCst);
        assert!(!poll_save_action());

        MENU_SAVE_CLICKED.store(true, Ordering::SeqCst);
        // Multiple concurrent reads would all see true until first swap
        let result = poll_save_action();
        assert!(result);
        // After swap, subsequent reads see false
        assert!(!poll_save_action());
    }

    #[test]
    fn test_poll_save_as_action_initially_false() {
        // Ensure the flag starts as false (it may have been set by a previous test)
        // Poll it to reset, then check again
        let _ = poll_save_as_action();
        assert!(!poll_save_as_action(), "poll_save_as_action should return false when no action triggered");
    }

    #[test]
    fn test_poll_save_as_action_resets_after_poll() {
        // Manually set the flag and verify polling resets it
        MENU_SAVE_AS_CLICKED.store(true, Ordering::SeqCst);
        assert!(poll_save_as_action(), "First poll should return true");
        assert!(!poll_save_as_action(), "Second poll should return false (flag was reset)");
    }

    #[test]
    fn test_poll_save_as_action_atomic_swap() {
        // Verify the atomic swap behavior
        MENU_SAVE_AS_CLICKED.store(false, Ordering::SeqCst);
        assert!(!poll_save_as_action());

        MENU_SAVE_AS_CLICKED.store(true, Ordering::SeqCst);
        // Multiple concurrent reads would all see true until first swap
        let result = poll_save_as_action();
        assert!(result);
        // After swap, subsequent reads see false
        assert!(!poll_save_as_action());
    }

    #[test]
    fn test_poll_export_images_action_initially_false() {
        // Ensure the flag starts as false (it may have been set by a previous test)
        // Poll it to reset, then check again
        let _ = poll_export_images_action();
        assert!(!poll_export_images_action(), "poll_export_images_action should return false when no action triggered");
    }

    #[test]
    fn test_poll_export_images_action_resets_after_poll() {
        // Manually set the flag and verify polling resets it
        MENU_EXPORT_IMAGES_CLICKED.store(true, Ordering::SeqCst);
        assert!(poll_export_images_action(), "First poll should return true");
        assert!(!poll_export_images_action(), "Second poll should return false (flag was reset)");
    }

    #[test]
    fn test_poll_export_images_action_atomic_swap() {
        // Verify the atomic swap behavior
        MENU_EXPORT_IMAGES_CLICKED.store(false, Ordering::SeqCst);
        assert!(!poll_export_images_action());

        MENU_EXPORT_IMAGES_CLICKED.store(true, Ordering::SeqCst);
        // Multiple concurrent reads would all see true until first swap
        let result = poll_export_images_action();
        assert!(result);
        // After swap, subsequent reads see false
        assert!(!poll_export_images_action());
    }
}
