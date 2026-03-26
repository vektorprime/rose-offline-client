# Name Tag Font Blurry Fix

## Date
2026-03-26

## Issue
Name tags displayed blurry fonts in the game.

## Root Causes Identified

1. **Glyph coverage extraction was fragile**: The code assumed alpha channel always contains glyph coverage
   ```rust
   // Old - fragile
   let coverage = if pixel[3] > 0 {
       pixel[3]
   } else {
       ((pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3) as u8
   };
   ```

2. **Outline algorithm created blurry halos**: The 2-pixel radius dilation created soft, blurry edges rather than crisp outlines

## Fix Applied

1. **Aligned with chat bubble behavior**: Use `max(r,g,b,a)` for robust coverage extraction
   ```rust
   // New - robust
   let coverage = pixel[0].max(pixel[1]).max(pixel[2]).max(pixel[3]);
   ```

2. **Removed outline entirely**: Eliminated the outline post-processing that was causing blurriness

## Files Modified
- `src/systems/name_tag_system.rs`

## Verification
Build successful, user confirmed fonts are no longer blurry.
