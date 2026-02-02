# RenderDoc Debugging Guide for rose-offline-client

This guide explains how to use RenderDoc to debug GPU rendering issues in the Bevy 0.14.2 application.

## What is RenderDoc?

RenderDoc is a free, open-source graphics debugger that captures frames and allows you to inspect:
- GPU draw calls and API usage
- Shader code and execution
- Mesh geometry and vertex buffers
- Textures and framebuffers
- Pipeline state and bindings

## Installation

1. Download RenderDoc from https://renderdoc.org/
2. Install it on your system
3. Launch RenderDoc

## Usage

### Method 1: Launch from RenderDoc (Recommended)

1. Open RenderDoc
2. Click **File → Launch Application**
3. Set the following:
   - **Executable Path**: Path to your compiled `rose-offline-client.exe`
   - **Working Directory**: The directory containing your `data.idx` and config files
   - **Environment Variables**: Add `RENDERDOC_CAPTURE=1`
4. Click **Launch**
5. Press **F12** to capture a frame while the app is running

### Method 2: Environment Variable

1. Set the environment variable before running:
   ```cmd
   set RENDERDOC_CAPTURE=1
   rose-offline-client.exe
   ```
2. Press **F12** to capture a frame

### Method 3: Manual Injection

1. Launch your application normally
2. Open RenderDoc
3. Click **File → Inject into Process**
4. Select the `rose-offline-client.exe` process
5. Capture frames using F12 or the RenderDoc UI

## Capture Analysis

### Event Browser

The Event Browser shows all GPU commands in the captured frame:

1. **Draw Calls**: Look for `vkCmdDraw*` or `Draw*` entries
   - Green entries indicate draw calls that rendered something
   - Gray entries indicate draws with no output (culled/empty)
   - If you see no draw calls, the scene isn't reaching the GPU

2. **Render Passes**: Look for render pass boundaries
   - Main scene renders typically happen in a render pass
   - Post-processing effects appear as separate passes

3. **Pipeline State**: Click any draw call to see:
   - Vertex shader inputs
   - Fragment shader outputs
   - Depth/stencil settings

### Pipeline State

Check the pipeline state for issues:

1. **Input Assembly**:
   - Are vertex buffers bound?
   - Is the index buffer valid?
   - Are vertex attributes correctly configured?

2. **Rasterizer State**:
   - Is culling enabled? (Check Cull Mode)
   - Is the viewport valid?
   - Is scissor testing interfering?

3. **Depth Stencil**:
   - Is depth testing enabled?
   - Is depth write enabled?
   - Is the depth function correct?

### Mesh Viewer

Verify geometry is reaching the GPU:

1. Select a draw call in the Event Browser
2. Switch to the **Mesh Viewer** tab
3. Check:
   - Are vertices in the expected positions?
   - Is the mesh properly formed (no NaN/infinity)?
   - Are indices valid?

### Texture Viewer

Check textures and render targets:

1. **Output Color**: Shows what the final image should be
   - If black, check earlier render passes
   - If unexpected colors, check shader output

2. **Depth Buffer**: 
   - Should show depth values (white = far, black = near)
   - If all white/black, depth testing may be wrong

3. **Individual Textures**:
   - Verify textures are loaded correctly
   - Check format and mip levels

## Common Issues and Debugging

### Black Screen

1. **Check for Draw Calls**:
   - Open Event Browser
   - Look for any draw calls
   - If none exist, the issue is CPU-side (culling, visibility, etc.)

2. **Check First Draw Call**:
   - Click the first draw call
   - Check Mesh Viewer - can you see geometry?
   - Check Texture Viewer - is output black?

3. **Check Clear Values**:
   - Look for `vkCmdBeginRenderPass` or similar
   - Verify clear color isn't black (or is intentional)

4. **Check Viewport and Scissor**:
   - In Pipeline State, verify viewport dimensions
   - Check scissor rect isn't zero-sized

### Missing Meshes

1. **Check Visibility**:
   - Look for draw calls in Event Browser
   - If draw calls exist but output is black, check:
     - Vertex positions (Mesh Viewer)
     - Shader outputs (Texture Viewer)

2. **Check Culling**:
   - In Pipeline State → Rasterizer State
   - Try disabling backface culling temporarily
   - Check winding order of triangles

3. **Check Transform**:
   - In Mesh Viewer, verify positions
   - Are meshes behind camera? (Z > 0 in view space)
   - Are meshes scaled incorrectly? (very large/small)

### Shader Issues

1. **Vertex Shader**:
   - Check input attributes match vertex buffer layout
   - Verify uniforms/matrices are correct
   - Check for division by zero or NaN

2. **Fragment Shader**:
   - Check texture sampling
   - Verify UV coordinates
   - Check alpha discard thresholds

## Debugging the Black Screen Issue

### Step 1: Verify Draw Calls Exist

1. Capture a frame
2. Open Event Browser
3. Look for `Draw*` or `vkCmdDraw*` entries
4. **If NO draw calls**: The issue is CPU-side
   - Check entity visibility
   - Check frustum culling
   - Check mesh/material components

### Step 2: Check First Draw Call

1. Select the first draw call
2. Open Mesh Viewer
3. **If mesh visible**: Issue is fragment shader or blending
4. **If mesh NOT visible**: Issue is vertex shader or vertex data

### Step 3: Check Output

1. Select the last draw call
2. Open Texture Viewer → Output Color
3. **If black**: Check depth/stencil state
4. **If not black but screen is black**: Check presentation/swapchain

### Step 4: Compare Working vs Non-Working

1. Capture a frame from Bevy examples (if they work)
2. Compare pipeline states
3. Look for differences in:
   - Render pass setup
   - Pipeline configuration
   - Descriptor sets/bindings

## Tips

1. **Use Markers**: The code includes `renderdoc_event_marker()` for custom markers
2. **Multiple Captures**: Capture several frames to see patterns
3. **Simplify**: Disable post-processing, UI, etc. to isolate the issue
4. **Compare**: Compare captures with working Bevy examples
5. **Check Errors**: Look for validation errors in RenderDoc's error log

## Troubleshooting

### "RenderDoc not found" Warning

This is normal if:
- You're not running from RenderDoc
- RenderDoc isn't injected
- The environment variable isn't set

The application will still run normally.

### No Captures Triggering

- Ensure `RENDERDOC_CAPTURE=1` is set
- Press F12 (default capture key)
- Check RenderDoc's capture key bindings

### Crashes When Capturing

- Disable validation layers if enabled
- Try software (WARP) renderer
- Update GPU drivers
- Check RenderDoc compatibility with your GPU

## Additional Resources

- RenderDoc Documentation: https://renderdoc.org/docs/
- RenderDoc Tutorials: https://renderdoc.org/tutorials/
- Bevy Render Documentation: https://bevyengine.org/learn/book/rendering/
- WGPU Debugging: https://github.com/gfx-rs/wgpu/wiki/Debugging

## Integration Details

The RenderDoc integration is implemented in:
- `src/debug/renderdoc.rs` - Main integration module
- `src/debug/mod.rs` - Debug module exports

Key features:
- Optional - doesn't require RenderDoc to run
- Environment variable controlled (`RENDERDOC_CAPTURE`)
- F12 key for frame capture
- Marker support for debugging specific systems
