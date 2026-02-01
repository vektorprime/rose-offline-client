import re
import sys

def parse_log(file_path):
    print(f"--- Parsing {file_path} ---")
    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"File {file_path} not found.")
        return

    important_keywords = [
        "ERROR", "WARN", 
        "zone_render_validation_system", 
        "aabb_diagnostic_system",
        "shader", "material", "mesh", "camera", "visibility",
        "AABB", "frustum", "culling", "black screen",
        "RENDER STATUS", "ACTIVE CAMERA", "VISIBILITY STATE",
        "MATERIAL DIAGNOSTICS", "RENDER PIPELINE DIAGNOSTICS",
        "AABB VALIDATION DIAGNOSTICS", "RENDER LAYER DIAGNOSTICS",
        "RENDER STAGE DIAGNOSTICS", "TRANSFORM PROPAGATION DIAGNOSTICS",
        "[ZONE VISIBILITY DEBUG]", "No entities are ready to render!"
    ]

    # Regex to match Bevy/Tracing log format: 2026-02-01T... LEVEL log: message
    # Or just look for keywords in lines
    
    for line in lines:
        if any(keyword in line for keyword in important_keywords):
            # Clean up ANSI escape codes if present
            clean_line = re.sub(r'\x1b\[[0-9;]*[mK]', '', line).strip()
            if clean_line:
                try:
                    print(clean_line)
                except UnicodeEncodeError:
                    print(clean_line.encode('ascii', 'ignore').decode('ascii'))

if __name__ == "__main__":
    parse_log("output.log")
    parse_log("error.log")
