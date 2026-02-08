//! Simple test to verify enigo mouse control works correctly

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Mouse, Settings};
use std::env;
use std::{thread, time::Duration};

fn parse_coords(s: &str) -> Option<(i32, i32)> {
    let mut parts = s.split(',');
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y))
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse optional target coordinates from command line
    let (target_x, target_y) = if args.len() >= 3 {
        let x: i32 = args[1].parse().unwrap_or(500);
        let y: i32 = args[2].parse().unwrap_or(500);
        (x, y)
    } else {
        (500, 500)
    };
    let mut do_click = false;
    let mut drag_to: Option<(i32, i32)> = None;
    let mut scroll_lines: i32 = 0;
    let mut steps: usize = 1;
    let mut step_delay_ms: u64 = 90;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--click" => {
                do_click = true;
            }
            "--drag-to" => {
                if i + 1 < args.len() {
                    drag_to = parse_coords(&args[i + 1]);
                    i += 1;
                }
            }
            "--scroll" => {
                if i + 1 < args.len() {
                    scroll_lines = args[i + 1].parse().unwrap_or(0);
                    i += 1;
                }
            }
            "--steps" => {
                if i + 1 < args.len() {
                    steps = args[i + 1].parse().unwrap_or(1).max(1);
                    i += 1;
                }
            }
            "--delay-ms" => {
                if i + 1 < args.len() {
                    step_delay_ms = args[i + 1].parse().unwrap_or(90).max(1);
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("Testing enigo mouse control...");
    println!(
        "Target: ({}, {}), Click: {}, DragTo: {:?}, Scroll(lines): {}, Steps: {}, Delay: {}ms",
        target_x,
        target_y,
        do_click,
        drag_to,
        scroll_lines,
        steps,
        step_delay_ms
    );

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

    if let Some((drag_x, drag_y)) = drag_to {
        println!("\nDragging to ({}, {})...", drag_x, drag_y);
        if let Err(e) = enigo.button(Button::Left, Direction::Press) {
            eprintln!("Failed to press mouse button: {:?}", e);
        } else {
            thread::sleep(Duration::from_millis(80));
            if let Err(e) = enigo.move_mouse(drag_x, drag_y, Coordinate::Abs) {
                eprintln!("Failed to move while dragging: {:?}", e);
            } else {
                thread::sleep(Duration::from_millis(120));
            }
            if let Err(e) = enigo.button(Button::Left, Direction::Release) {
                eprintln!("Failed to release mouse button: {:?}", e);
            } else {
                println!("✓ Drag command sent");
            }
            thread::sleep(Duration::from_millis(120));
        }
    }

    if scroll_lines != 0 {
        println!("\nScrolling...");
        for step in 0..steps {
            if scroll_lines != 0 {
                if let Err(e) = enigo.scroll(scroll_lines, Axis::Vertical) {
                    eprintln!("Failed line scroll step {}: {:?}", step + 1, e);
                }
            }
            thread::sleep(Duration::from_millis(step_delay_ms));
        }
        println!("✓ Scroll sequence sent");
    }

    println!("\nDone!");
}
