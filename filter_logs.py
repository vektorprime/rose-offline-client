import sys
import re

def filter_log(input_file, output_file):
    # Try to read as UTF-16 first, then fallback to UTF-8
    try:
        with open(input_file, 'r', encoding='utf-16') as f:
            lines = f.readlines()
    except (UnicodeDecodeError, UnicodeError):
        with open(input_file, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()
    
    filtered_lines = []
    for line in lines:
        # Remove ANSI escape sequences
        line = re.sub(r'\x1b\[[0-9;]*[mK]', '', line)

        # Skip empty lines (lines that are just whitespace or empty)
        if not line.strip():
            continue

        # Skip lines with timestamp + DEBUG pattern (e.g., "2026-02-02T04:27:07.407063Z DEBUG naga::front:")
        if re.match(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z\s+DEBUG\s+', line):
            continue

        # Find the content after "rose_offline_client:" prefix
        # This handles both "rose_offline_client: [TAG]" and "rose_offline_client:   - item" formats
        # Use rfind to get the LAST occurrence of "rose_offline_client:" in the line
        prefix = 'rose_offline_client:'
        idx = line.rfind(prefix)
        if idx != -1:
            # Get everything after the prefix
            content = line[idx + len(prefix):]
            # Remove any leading whitespace/colons from the content
            content = content.lstrip(': \t')
            # Skip if the resulting content is empty
            if not content:
                continue
            filtered_lines.append(content + '\n')
            continue

        # Fallback: if no prefix found, check if it's already a trimmed line
        # (for lines that don't have the full prefix pattern)
        filtered_lines.append(line)
            
    with open(output_file, 'w', encoding='utf-8') as f:
        f.writelines(filtered_lines)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python filter_logs.py <input_file> <output_file>")
    else:
        filter_log(sys.argv[1], sys.argv[2])

