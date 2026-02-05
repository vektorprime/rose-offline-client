import os
import sys
import json
import logging
from pathlib import Path
from openai import OpenAI
from qdrant_client import QdrantClient

# -----------------------
# LOGGING SETUP
# -----------------------
logging.basicConfig(
    filename='mcp-debug.log',
    level=logging.DEBUG,
    format='%(asctime)s - %(message)s'
)
logging.debug("MCP server starting...")

# -----------------------
# CONFIG
# -----------------------
QDRANT_URL = os.environ.get("QDRANT_URL", "http://127.0.0.1:6333")
COLLECTION = os.environ.get("COLLECTION_NAME", "bevy-0-14-2")
EMBEDDING_MODEL = os.environ.get("EMBEDDING_MODEL", "text-embedding-embeddinggemma-300m@bf16")
LM_STUDIO_URL = os.environ.get("LM_STUDIO_URL", "http://127.0.0.1:1234/v1")
LM_STUDIO_API_KEY = os.environ.get("LM_STUDIO_API_KEY", "lm-studio")
BEVY_SOURCE_PATH = os.environ.get("BEVY_SOURCE_PATH", "C:\\path\\to\\bevy-0.14.2")
MIN_RELEVANCE_SCORE = float(os.environ.get("MIN_RELEVANCE_SCORE", "0.5"))  # Filter low scores

logging.debug(f"Config - QDRANT_URL: {QDRANT_URL}")
logging.debug(f"Config - COLLECTION: {COLLECTION}")
logging.debug(f"Config - EMBEDDING_MODEL: {EMBEDDING_MODEL}")
logging.debug(f"Config - LM_STUDIO_URL: {LM_STUDIO_URL}")
logging.debug(f"Config - BEVY_SOURCE_PATH: {BEVY_SOURCE_PATH}")
logging.debug(f"Config - MIN_RELEVANCE_SCORE: {MIN_RELEVANCE_SCORE}")

# -----------------------
# CLIENTS
# -----------------------
try:
    lm_client = OpenAI(base_url=LM_STUDIO_URL, api_key=LM_STUDIO_API_KEY)
    qdrant = QdrantClient(url=QDRANT_URL, prefer_grpc=False)
    logging.debug("Clients initialized successfully")
except Exception as e:
    logging.error(f"Failed to initialize clients: {e}")
    raise

# -----------------------
# FILE FILTERING
# -----------------------
def is_relevant_file(file_path: str, query: str) -> tuple[bool, str]:
    """
    Determine if a file is likely to contain relevant information.
    Returns (is_relevant, reason)
    """
    path_lower = file_path.lower()
    query_lower = query.lower()
    
    # Prioritize core ECS and API files over examples
    priority_paths = [
        'crates/bevy_ecs',
        'crates/bevy_app',
        'crates/bevy_core',
        'src/',
    ]
    
    # Deprioritize but don't exclude examples
    example_paths = ['examples/', 'benches/', 'tests/']
    
    # Extract key terms from query
    ecs_terms = ['entity', 'component', 'system', 'query', 'resource', 'world', 'commands']
    rendering_terms = ['render', 'mesh', 'material', 'shader', 'camera', 'light']
    
    is_priority = any(p in path_lower for p in priority_paths)
    is_example = any(p in path_lower for p in example_paths)
    
    # Check if query terms align with file path
    query_has_ecs = any(term in query_lower for term in ecs_terms)
    query_has_rendering = any(term in query_lower for term in rendering_terms)
    
    if is_priority and not is_example:
        return True, "core API file"
    elif is_example and (query_has_ecs or query_has_rendering):
        return True, "example file (may contain usage patterns)"
    elif is_example:
        return False, "example file (low priority for this query)"
    else:
        return True, "potentially relevant"

# -----------------------
# FILE READING WITH CONTEXT EXTRACTION
# -----------------------
def read_file_content(file_path: str, query: str, max_chars: int = 3000) -> str:
    """
    Read file content with smart extraction around relevant sections.
    """
    try:
        full_path = Path(BEVY_SOURCE_PATH) / file_path
        logging.debug(f"Reading file: {full_path}")
        
        if not full_path.exists():
            logging.warning(f"File not found: {full_path}")
            return f"*File not found at {full_path}*"
        
        with open(full_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Try to find relevant sections based on query terms
        query_terms = query.lower().split()
        lines = content.split('\n')
        
        # Find lines containing query terms
        relevant_line_indices = []
        for i, line in enumerate(lines):
            line_lower = line.lower()
            if any(term in line_lower for term in query_terms):
                relevant_line_indices.append(i)
        
        # If we found relevant sections, extract context around them
        if relevant_line_indices and len(content) > max_chars:
            # Get context window around relevant lines (±20 lines)
            context_lines = set()
            for idx in relevant_line_indices[:5]:  # Limit to first 5 matches
                start = max(0, idx - 20)
                end = min(len(lines), idx + 20)
                context_lines.update(range(start, end))
            
            # Sort and extract
            sorted_lines = sorted(context_lines)
            extracted_content = '\n'.join(lines[i] for i in sorted_lines)
            
            if len(extracted_content) < max_chars:
                return extracted_content
            else:
                return extracted_content[:max_chars] + f"\n\n... (truncated)"
        
        # Otherwise just truncate from the beginning
        if len(content) > max_chars:
            content = content[:max_chars] + f"\n\n... (truncated, {len(content) - max_chars} more characters)"
        
        return content
        
    except Exception as e:
        logging.error(f"Error reading file {file_path}: {e}")
        return f"*Error reading file: {e}*"

# -----------------------
# EMBEDDING
# -----------------------
def embed(text: str):
    logging.debug(f"Embedding text: {text[:100]}...")
    response = lm_client.embeddings.create(model=EMBEDDING_MODEL, input=text)
    return response.data[0].embedding

# -----------------------
# QUERY FUNCTION WITH FILTERING
# -----------------------
def query_qdrant(text: str, top_k: int = 5):
    logging.debug(f"Querying Qdrant with text: {text[:100]}, top_k: {top_k}")
    vector = embed(text)
    
    # Request more results than needed so we can filter
    search_limit = top_k * 3
    
    try:
        results = qdrant.query_points(
            collection_name=COLLECTION,
            query=vector,
            limit=search_limit
        ).points
        logging.debug(f"query_points successful, got {len(results)} results")
    except AttributeError:
        logging.debug("query_points failed, trying search")
        try:
            results = qdrant.search(
                collection_name=COLLECTION,
                query_vector=vector,
                limit=search_limit
            )
            logging.debug(f"search successful, got {len(results)} results")
        except AttributeError:
            logging.debug("search failed, trying search_points")
            results = qdrant.search_points(
                collection_name=COLLECTION,
                query_vector=vector,
                limit=search_limit
            )
            logging.debug(f"search_points successful, got {len(results)} results")
    
    # Filter and rank results
    formatted_results = []
    for r in results:
        file_path = r.payload.get("path", "Unknown")
        score = r.score
        
        # Skip low-relevance results
        if score < MIN_RELEVANCE_SCORE:
            logging.debug(f"Skipping {file_path} - score {score} below threshold")
            continue
        
        # Check if file is relevant based on path and query
        is_relevant, reason = is_relevant_file(file_path, text)
        
        if is_relevant:
            # Read the actual file content with smart extraction
            content = read_file_content(file_path, text)
            
            result_data = {
                "path": file_path,
                "score": score,
                "content": content,
                "relevance_reason": reason
            }
            
            formatted_results.append(result_data)
            
            # Stop when we have enough relevant results
            if len(formatted_results) >= top_k:
                break
        else:
            logging.debug(f"Filtered out {file_path}: {reason}")
    
    logging.debug(f"Returning {len(formatted_results)} filtered results")
    return formatted_results

# -----------------------
# MCP REQUEST HANDLERS
# -----------------------
def handle_initialize(request):
    logging.debug("Handling initialize request")
    return {
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "query-qdrant",
            "version": "1.0.0"
        }
    }

def handle_list_tools(request):
    logging.debug("Handling list_tools request")
    return {
        "tools": [
            {
                "name": "query_qdrant",
                "description": """Search the Bevy 0.14.2 source code using semantic search with intelligent filtering.

Use this tool when:
- User asks about Bevy ECS, entities, components, systems, resources, or queries
- You need to verify Bevy 0.14.2 specific APIs, syntax, or behavior
- Initial solutions didn't work and you need to check actual implementation
- User explicitly requests documentation lookup

Returns: Relevant Bevy source files with actual code content, prioritizing core API files over examples.

Query tips:
- Use specific terms: "spawn entity", "Query<&Transform>", "Commands.insert()"  
- Include context: "mutable component query", "startup system"
- For errors: "borrow checker error Query"

Note: Results are filtered for relevance - you'll only see files likely to contain useful information.""",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Semantic search query using specific Bevy terms"
                        },
                        "top_k": {
                            "type": "number",
                            "description": "Number of results to return (default: 5)",
                            "default": 5
                        }
                    },
                    "required": ["query"]
                }
            }
        ]
    }

def handle_call_tool(request):
    logging.debug("Handling call_tool request")
    tool_name = request.get("params", {}).get("name")
    arguments = request.get("params", {}).get("arguments", {})
    
    logging.debug(f"Tool name: {tool_name}, arguments: {arguments}")
    
    if tool_name == "query_qdrant":
        query_text = arguments.get("query", "")
        top_k = arguments.get("top_k", 5)
        
        results = query_qdrant(query_text, top_k)
        
        if not results:
            return {
                "content": [{
                    "type": "text",
                    "text": "No relevant results found. Try:\n- Using more specific Bevy terminology\n- Rephrasing your query\n- Searching for related concepts"
                }]
            }
        
        # Format results with actual file content
        content = []
        for i, result in enumerate(results, 1):
            result_text = f"**Result {i}** (relevance: {result['score']:.4f}) - {result['relevance_reason']}\n"
            result_text += f"**File**: `{result['path']}`\n\n"
            result_text += f"**Content**:\n```rust\n{result['content']}\n```\n\n"
            result_text += "---\n\n"
            
            content.append({
                "type": "text",
                "text": result_text
            })
        
        logging.debug(f"Returning {len(content)} results")
        return {
            "content": content
        }
    else:
        raise ValueError(f"Unknown tool: {tool_name}")

# -----------------------
# MCP LOOP
# -----------------------
def mcp_loop():
    logging.debug("Entering MCP loop")
    while True:
        line = None
        req_id = 1
        
        try:
            line = sys.stdin.readline()
            if not line:
                logging.debug("EOF received, exiting")
                break
            
            logging.debug(f"Received line: {line.strip()}")
            
            request = json.loads(line)
            req_id = request.get("id", 1)
            method = request.get("method")
            
            logging.debug(f"Request ID: {req_id}, Method: {method}")
            
            # Handle notifications (no response needed)
            if method and method.startswith("notifications/"):
                logging.debug(f"Received notification: {method}, ignoring")
                continue
            
            # Handle regular requests
            if method == "initialize":
                result = handle_initialize(request)
            elif method == "tools/list":
                result = handle_list_tools(request)
            elif method == "tools/call":
                result = handle_call_tool(request)
            else:
                raise ValueError(f"Unknown method: {method}")
            
            response = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": result
            }
            
            response_json = json.dumps(response)
            logging.debug(f"Sending response: {response_json}")
            print(response_json, flush=True)
            
        except json.JSONDecodeError as e:
            logging.error(f"JSON decode error: {e}", exc_info=True)
            continue
            
        except Exception as e:
            logging.error(f"Error in MCP loop: {e}", exc_info=True)
            response = {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32000,
                    "message": str(e)
                }
            }
            response_json = json.dumps(response)
            logging.debug(f"Sending error response: {response_json}")
            print(response_json, flush=True)

if __name__ == "__main__":
    logging.debug("Starting MCP server main")
    try:
        mcp_loop()
    except Exception as e:
        logging.error(f"Fatal error: {e}", exc_info=True)
        raise