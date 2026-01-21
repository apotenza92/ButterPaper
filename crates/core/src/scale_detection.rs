//! Scale detection from OCR text
//!
//! Parses OCR text to detect scale notations commonly found on
//! construction drawings and engineering plans.

use crate::measurement::ScaleSystem;
use std::collections::HashMap;

/// A detected scale from OCR text
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedScale {
    /// Detected ratio (page units per real-world unit)
    pub ratio: f32,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Detected unit (e.g., "m", "ft", "mm")
    pub unit: String,
    /// Source text that was parsed
    pub source_text: String,
    /// Location in text where scale was found
    pub text_offset: usize,
}

impl DetectedScale {
    /// Create a new detected scale
    pub fn new(
        ratio: f32,
        confidence: f32,
        unit: impl Into<String>,
        source_text: impl Into<String>,
        text_offset: usize,
    ) -> Self {
        Self {
            ratio,
            confidence,
            unit: unit.into(),
            source_text: source_text.into(),
            text_offset,
        }
    }

    /// Convert to a ScaleSystem for a specific page
    pub fn to_scale_system(&self, page_index: u16) -> ScaleSystem {
        ScaleSystem::ocr_detected(page_index, self.ratio, self.confidence, &self.unit)
    }
}

/// Parse OCR text to detect scale notations
///
/// Returns a list of detected scales sorted by confidence (highest first).
/// Common formats detected:
/// - "1:100" (metric ratio)
/// - "Scale 1:50"
/// - "1/4\" = 1'-0\"" (imperial architectural)
/// - "1\" = 20'" (imperial engineering)
/// - "1/8\" = 1'-0\""
pub fn detect_scales(text: &str) -> Vec<DetectedScale> {
    let mut detections = Vec::new();

    // Pattern 1: Simple ratio notation (1:100, 1:50, etc.)
    detections.extend(detect_simple_ratio(text));

    // Pattern 2: Metric scale with unit (1:100 m, Scale 1:50, etc.)
    detections.extend(detect_metric_scale(text));

    // Pattern 3: Imperial architectural scale (1/4" = 1'-0")
    detections.extend(detect_imperial_architectural(text));

    // Pattern 4: Imperial engineering scale (1" = 20')
    detections.extend(detect_imperial_engineering(text));

    // Sort by confidence (highest first)
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    detections
}

/// Detect simple ratio notation (1:100)
fn detect_simple_ratio(text: &str) -> Vec<DetectedScale> {
    let mut detections = Vec::new();
    let text_lower = text.to_lowercase();

    // Look for patterns like "1:100", "1:50", "scale 1:200"
    for (offset, line) in text_lower.lines().enumerate() {
        // Match "1:N" where N is a number
        for (idx, window) in line.as_bytes().windows(2).enumerate() {
            if window[0] == b'1' && window[1] == b':' {
                // Extract the number after the colon
                let rest = &line[idx + 2..];
                if let Some(end_idx) = rest.find(|c: char| !c.is_ascii_digit()) {
                    let number_str = &rest[..end_idx];
                    if let Ok(denominator) = number_str.parse::<f32>() {
                        if denominator > 0.0 && denominator <= 10000.0 {
                            // Reasonable scale range
                            let ratio = denominator; // 1:N means N page units per 1 real unit
                            let confidence = if line.contains("scale") { 0.9 } else { 0.7 };
                            let source = format!("1:{}", denominator);

                            // Infer unit based on typical scale ranges
                            let unit = if denominator >= 100.0 {
                                "m" // Large scales typically metric
                            } else if denominator >= 10.0 {
                                "ft" // Medium scales could be imperial
                            } else {
                                "m" // Default to metric
                            };

                            detections.push(DetectedScale::new(
                                ratio,
                                confidence,
                                unit,
                                source,
                                offset * 100 + idx,
                            ));
                        }
                    }
                } else if !rest.is_empty() {
                    // Number goes to end of line
                    if let Ok(denominator) = rest.parse::<f32>() {
                        if denominator > 0.0 && denominator <= 10000.0 {
                            let ratio = denominator;
                            let confidence = if line.contains("scale") { 0.9 } else { 0.7 };
                            let source = format!("1:{}", denominator);
                            let unit = if denominator >= 100.0 {
                                "m"
                            } else if denominator >= 10.0 {
                                "ft"
                            } else {
                                "m"
                            };

                            detections.push(DetectedScale::new(
                                ratio,
                                confidence,
                                unit,
                                source,
                                offset * 100 + idx,
                            ));
                        }
                    }
                }
            }
        }
    }

    detections
}

/// Detect metric scale with explicit unit
fn detect_metric_scale(text: &str) -> Vec<DetectedScale> {
    let mut detections = Vec::new();
    let text_lower = text.to_lowercase();

    // Unit conversion table: maps common unit names to standard abbreviations
    let units = [
        ("meter", "m"),
        ("meters", "m"),
        ("metre", "m"),
        ("metres", "m"),
        ("centimeter", "cm"),
        ("centimeters", "cm"),
        ("centimetre", "cm"),
        ("centimetres", "cm"),
        ("millimeter", "mm"),
        ("millimeters", "mm"),
        ("millimetre", "mm"),
        ("millimetres", "mm"),
    ];

    for (offset, line) in text_lower.lines().enumerate() {
        // Look for "1:N [unit]" or "scale 1:N [unit]"
        if let Some(colon_idx) = line.find("1:") {
            let after_colon = &line[colon_idx + 2..];
            if let Some(space_idx) = after_colon.find(char::is_whitespace) {
                let number_part = &after_colon[..space_idx];
                let unit_part = &after_colon[space_idx..].trim();

                if let Ok(denominator) = number_part.parse::<f32>() {
                    if denominator > 0.0 && denominator <= 10000.0 {
                        // Check if unit is present
                        for (long_name, short_name) in &units {
                            if unit_part.starts_with(long_name) {
                                let ratio = denominator;
                                let confidence = 0.95; // High confidence when unit is explicit
                                let source = format!("1:{} {}", denominator, long_name);

                                detections.push(DetectedScale::new(
                                    ratio,
                                    confidence,
                                    *short_name,
                                    source,
                                    offset * 100 + colon_idx,
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    detections
}

/// Detect imperial architectural scale (e.g., 1/4" = 1'-0")
fn detect_imperial_architectural(text: &str) -> Vec<DetectedScale> {
    let mut detections = Vec::new();

    // Common architectural scales mapping
    let arch_scales: HashMap<&str, f32> = [
        ("1/16", 192.0), // 1/16" = 1'-0" means 192 units per foot
        ("1/8", 96.0),   // 1/8" = 1'-0" means 96 units per foot
        ("3/16", 64.0),  // 3/16" = 1'-0"
        ("1/4", 48.0),   // 1/4" = 1'-0" means 48 units per foot
        ("3/8", 32.0),   // 3/8" = 1'-0"
        ("1/2", 24.0),   // 1/2" = 1'-0" means 24 units per foot
        ("3/4", 16.0),   // 3/4" = 1'-0"
        ("1", 12.0),     // 1" = 1'-0" means 12 units per foot
        ("1-1/2", 8.0),  // 1-1/2" = 1'-0"
        ("3", 4.0),      // 3" = 1'-0" means 4 units per foot
    ]
    .iter()
    .copied()
    .collect();

    for (offset, line) in text.lines().enumerate() {
        for (fraction, ratio) in &arch_scales {
            // Look for patterns like "1/4\" = 1'-0\"" or "scale 1/4\" = 1'"
            let pattern1 = format!("{}\" = 1'-0\"", fraction);
            let pattern2 = format!("{}\" = 1'", fraction);
            let pattern3 = format!("{}\"=1'-0\"", fraction); // No spaces
            let pattern4 = format!("{}\"=1'", fraction);

            for pattern in &[pattern1, pattern2, pattern3, pattern4] {
                if line.contains(pattern) {
                    detections.push(DetectedScale::new(
                        *ratio,
                        0.95, // High confidence for standard architectural scales
                        "ft",
                        pattern.clone(),
                        offset * 100,
                    ));
                    break;
                }
            }
        }
    }

    detections
}

/// Detect imperial engineering scale (e.g., 1" = 20')
fn detect_imperial_engineering(text: &str) -> Vec<DetectedScale> {
    let mut detections = Vec::new();

    for (offset, line) in text.lines().enumerate() {
        // Look for patterns like "1\" = N'" where N is a number
        if let Some(idx) = line.find("1\" =") {
            let after_eq = &line[idx + 4..].trim_start();

            // Try to extract the number before the apostrophe
            if let Some(apos_idx) = after_eq.find('\'') {
                let number_str = after_eq[..apos_idx].trim();
                if let Ok(feet) = number_str.parse::<f32>() {
                    if feet > 0.0 && feet <= 1000.0 {
                        // 1" represents 'feet' feet
                        // If we assume 12 units = 1 inch, then ratio = feet * 12
                        let ratio = feet * 12.0;
                        let confidence = 0.9;
                        let source = format!("1\" = {}'", feet);

                        detections.push(DetectedScale::new(
                            ratio,
                            confidence,
                            "ft",
                            source,
                            offset * 100 + idx,
                        ));
                    }
                }
            }
        }

        // Also check without spaces: 1"=N'
        if let Some(idx) = line.find("1\"=") {
            let after_eq = &line[idx + 3..];

            if let Some(apos_idx) = after_eq.find('\'') {
                let number_str = after_eq[..apos_idx].trim();
                if let Ok(feet) = number_str.parse::<f32>() {
                    if feet > 0.0 && feet <= 1000.0 {
                        let ratio = feet * 12.0;
                        let confidence = 0.9;
                        let source = format!("1\"={}'", feet);

                        detections.push(DetectedScale::new(
                            ratio,
                            confidence,
                            "ft",
                            source,
                            offset * 100 + idx,
                        ));
                    }
                }
            }
        }
    }

    detections
}

/// Get the best (highest confidence) detected scale from text
pub fn get_best_scale(text: &str) -> Option<DetectedScale> {
    let scales = detect_scales(text);
    scales.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_ratio_detection() {
        let text = "Drawing Scale 1:100";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        let best = &scales[0];
        assert_eq!(best.ratio, 100.0);
        assert!(best.confidence > 0.8);
        assert_eq!(best.unit, "m");
    }

    #[test]
    fn test_metric_scale_with_unit() {
        let text = "Scale 1:50 meters";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        let best = &scales[0];
        assert_eq!(best.ratio, 50.0);
        assert!(best.confidence > 0.9);
        assert_eq!(best.unit, "m");
    }

    #[test]
    fn test_imperial_architectural() {
        let text = "Scale: 1/4\" = 1'-0\"";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        let best = &scales[0];
        assert_eq!(best.ratio, 48.0);
        assert!(best.confidence > 0.9);
        assert_eq!(best.unit, "ft");
    }

    #[test]
    fn test_imperial_engineering() {
        let text = "Scale: 1\" = 20'";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        let best = &scales[0];
        assert_eq!(best.ratio, 240.0); // 20 feet * 12 inches/foot
        assert!(best.confidence > 0.8);
        assert_eq!(best.unit, "ft");
    }

    #[test]
    fn test_multiple_scales() {
        let text = "Main Floor: Scale 1:100\nDetail: Scale 1:50";
        let scales = detect_scales(text);

        assert!(scales.len() >= 2);
        // Should detect both scales
        assert!(scales.iter().any(|s| s.ratio == 100.0));
        assert!(scales.iter().any(|s| s.ratio == 50.0));
    }

    #[test]
    fn test_no_scale_detected() {
        let text = "This is just some text without any scale information";
        let scales = detect_scales(text);

        assert!(scales.is_empty());
    }

    #[test]
    fn test_get_best_scale() {
        let text = "Scale 1:100 meters\nNote: 1:50";
        let best = get_best_scale(text);

        assert!(best.is_some());
        let scale = best.unwrap();
        // Should prefer the one with explicit unit (higher confidence)
        assert_eq!(scale.ratio, 100.0);
        assert_eq!(scale.unit, "m");
    }

    #[test]
    fn test_architectural_scale_variants() {
        let variants = vec![
            "1/8\" = 1'-0\"",
            "1/8\" = 1'",
            "1/8\"=1'-0\"",
            "1/8\"=1'",
        ];

        for variant in variants {
            let scales = detect_scales(variant);
            assert!(
                !scales.is_empty(),
                "Failed to detect scale in: {}",
                variant
            );
            assert_eq!(scales[0].ratio, 96.0);
            assert_eq!(scales[0].unit, "ft");
        }
    }

    #[test]
    fn test_engineering_scale_variants() {
        let text = "1\" = 30'";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        assert_eq!(scales[0].ratio, 360.0); // 30 * 12
        assert_eq!(scales[0].unit, "ft");
    }

    #[test]
    fn test_case_insensitive() {
        let text = "SCALE 1:100 METERS";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        assert_eq!(scales[0].ratio, 100.0);
        assert_eq!(scales[0].unit, "m");
    }

    #[test]
    fn test_millimeter_unit() {
        let text = "Scale 1:20 millimeters";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        assert_eq!(scales[0].ratio, 20.0);
        assert_eq!(scales[0].unit, "mm");
    }

    #[test]
    fn test_centimeter_unit() {
        let text = "1:5 centimetres";
        let scales = detect_scales(text);

        assert!(!scales.is_empty());
        assert_eq!(scales[0].ratio, 5.0);
        assert_eq!(scales[0].unit, "cm");
    }

    #[test]
    fn test_invalid_ratios_ignored() {
        let text = "1:0 1:99999 1:-5"; // Invalid ratios
        let scales = detect_scales(text);

        // Should not detect any invalid scales
        for scale in scales {
            assert!(scale.ratio > 0.0);
            assert!(scale.ratio <= 10000.0);
        }
    }

    #[test]
    fn test_to_scale_system() {
        let detected = DetectedScale::new(100.0, 0.9, "m", "1:100 meters", 0);
        let scale_system = detected.to_scale_system(0);

        assert_eq!(scale_system.page_index(), 0);
        assert_eq!(scale_system.unit(), "m");
        assert_eq!(scale_system.ratio(), 100.0);
        assert!(scale_system.is_reliable()); // Confidence 0.9 > 0.8, should be reliable

        let detected_low = DetectedScale::new(100.0, 0.5, "m", "1:100", 0);
        let scale_system_low = detected_low.to_scale_system(0);
        assert!(!scale_system_low.is_reliable()); // Low confidence
    }
}
