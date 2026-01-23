//! Simple test to verify enigo mouse control works correctly

use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use std::env;
use std::{thread, time::Duration};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse optional target coordinates from command line
    let (target_x, target_y, do_click) = if args.len() >= 3 {
        let x: i32 = args[1].parse().unwrap_or(500);
        let y: i32 = args[2].parse().unwrap_or(500);
        let click = args.get(3).map(|s| s == "--click").unwrap_or(false);
        (x, y, click)
    } else {
        (500, 500, false)
    };

    println!("Testing enigo mouse control...");
    println!("Target: ({}, {}), Click: {}", target_x, target_y, do_click);

    let settings = Settings::default();

    let mut enigo = match Enigo::new(&settings) {
        Ok(e) => {
            println!("✓ Enigo created successfully");
            e
        }
        Err(e) => {
            eprintln!("✗ Failed to create enigo: {:?}", e);
            eprintln!("  Ensure Accessibility permissions are granted in:");
            eprintln!("  System Settings > Privacy & Security > Accessibility");
            std::process::exit(1);
        }
    };

    // Get current position
    match enigo.location() {
        Ok((x, y)) => println!("Current mouse position: ({}, {})", x, y),
        Err(e) => eprintln!("Failed to get mouse position: {:?}", e),
    }

    // Move to target position
    println!("\nMoving mouse to ({}, {})...", target_x, target_y);

    if let Err(e) = enigo.move_mouse(target_x, target_y, Coordinate::Abs) {
        eprintln!("Failed to move mouse: {:?}", e);
    } else {
        println!("✓ Move command sent");
    }

    // Delay to let the move complete
    thread::sleep(Duration::from_millis(100));

    // Verify new position
    match enigo.location() {
        Ok((x, y)) => {
            println!("Mouse position after move: ({}, {})", x, y);
            let dx = (x - target_x).abs();
            let dy = (y - target_y).abs();
            if dx <= 1 && dy <= 1 {
                println!("✓ Position is correct!");
            } else {
                println!(
                    "✗ Position mismatch! Expected ({}, {}), got ({}, {})",
                    target_x, target_y, x, y
                );
            }
        }
        Err(e) => eprintln!("Failed to get mouse position: {:?}", e),
    }

    // Optionally click
    if do_click {
        println!("\nClicking at ({}, {})...", target_x, target_y);

        // Try click
        if let Err(e) = enigo.button(Button::Left, Direction::Click) {
            eprintln!("Failed to click: {:?}", e);
        } else {
            println!("✓ Click command sent");
        }

        thread::sleep(Duration::from_millis(100));
    }

    println!("\nDone!");
}
