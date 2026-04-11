# Blood Effects Pitfalls

## Terrain Blood Splatter Invisible or Grass-Like (Fixed 2026-04-09)

### Problem
Blood splatter on terrain either appeared as thin grass-like lines or was effectively invisible.

### Root Cause
- Decal orientation used `looking_to(Vec3::NEG_Y, normal)` with an unsuitable transform path.
- Decal depth blending was too aggressive for terrain use.
- Procedural texture generation had low-contrast alpha/color distribution.

### Solution
- Built decals with explicit surface-normal alignment + tangent-plane spin.
- Kept decals very close to surface with a small normal offset.
- Raised minimum effective opacity and depth fade floor.
- Reworked procedural texture generation for high-contrast red/dark-red splatter + random spots.

### Files Modified
- `src/systems/blood_spatter_system.rs`

### Lesson Learned
For forward decals, solve visibility first through transform/orientation correctness and depth-fade tuning before increasing intensity.

## Monster Wound Blood Too Faint on Model (Fixed 2026-04-09)

### Problem
Blood on monster models was too faint/small and hard to notice.

### Root Cause
- Wound quads were too small for gameplay readability.
- Local joint-space offset compression reduced effective size.
- Overlay alpha/intensity was too low.

### Solution
- Increased minimum wound size and final wound quad scale.
- Reduced local pose compression on skinned meshes.
- Increased wound alpha and added subtle dark-red emissive tint.
- Increased normal offset to reduce z-fighting on mesh surfaces.

### Files Modified
- `src/systems/gash_wound_system.rs`

### Lesson Learned
Readable combat feedback overlays need explicit minimum visual thresholds (size/alpha) and anti-z-fight offsets.
