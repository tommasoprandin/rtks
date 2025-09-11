#!/bin/bash

BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m'
DARK_GRAY='\033[1;30m'
RED='\033[0;31m'

echo -e "${BLUE}Select the type of configuration you want to generate: "
echo -e "1) Holistic"
echo -e "2) Offset based"

read -rp "Enter your choice (1 or 2): " choice

mkdir -p results

case $choice in 
        1)
                OUTPUT_FILE="results/mast_holistic_configuration.txt"
                rm -f "$OUTPUT_FILE"

                find . -type f -name "*.txt" ! -name "*offset_transactions.txt" ! -name "*configuration.txt" | while read -r file; do
                echo -e "${DARK_GRAY}Processing $file...${NC}"
                cat "$file" >> "$OUTPUT_FILE"
                echo -e "\n" >> "$OUTPUT_FILE"
                done

                echo -e "${GREEN}Mast configuration correctly generated in $OUTPUT_FILE${NC}."
                ;;
        2)
                OUTPUT_FILE="results/mast_offset_based_configuration.txt"
                rm -f "$OUTPUT_FILE"

                find . -type f -name "*.txt" ! -name "*holistic_transactions.txt" ! -name "*configuration.txt" | while read -r file; do
                echo -e "${DARK_GRAY}Processing $file...${NC}"
                cat "$file" >> "$OUTPUT_FILE"
                echo -e "\n" >> "$OUTPUT_FILE"
                done

                echo -e "${GREEN}Mast configuration correctly generated in $OUTPUT_FILE${NC}."
                ;;
        *)
                echo -e "${RED}Invalid choice. Please run the script again and select either 1 or 2.${NC}"
                exit 1
                ;;
esac