#!/usr/bin/env python3
"""
Convert Solana keypair JSON to base58 private key
Usage: python3 keypair_to_base58.py <path_to_keypair.json>
"""

import json
import sys
import base58

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 keypair_to_base58.py <path_to_keypair.json>")
        sys.exit(1)
    
    keypair_path = sys.argv[1]
    
    try:
        with open(keypair_path, 'r') as f:
            keypair = json.load(f)
        
        # Convert to bytes and then to base58
        private_key_bytes = bytes(keypair)
        private_key_base58 = base58.b58encode(private_key_bytes).decode('utf-8')
        
        print(private_key_base58)
    
    except FileNotFoundError:
        print(f"Error: File not found: {keypair_path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Error: Invalid JSON in {keypair_path}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
