#!/bin/bash

OUTPUT_FILE="mast_configuration.txt"

rm -f "$OUTPUT_FILE"

GREEN='\033[0;32m'
NC='\033[0m'
DARK_GRAY='\033[1;30m'

find . -type f -name "*.txt" | while read -r file; do
        echo -e "${DARK_GRAY}Processing $file...${NC}"
        cat "$file" >> "$OUTPUT_FILE"
        echo -e "\n" >> "$OUTPUT_FILE"
done

echo -e "${GREEN}Mast configuration correctly generated in $OUTPUT_FILE${NC}."