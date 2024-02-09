OUT_DIR=./single-html
DIST_DIR=./dist
OUT_FILE=index.html

all: $(OUT_DIR)/$(OUT_FILE)

$(OUT_DIR)/$(OUT_FILE) $(OUT_DIR)/encoded: $(OUT_DIR) $(DIST_DIR)/index.html
	# Inline css and js
	python prepare_single_file.py -i $(DIST_DIR)/index.html -s $(DIST_DIR)/assets -o $(OUT_DIR) > $@
	# Another pass of trimming
	inliner $@ > $@.tmp && mv $@.tmp $@

$(OUT_DIR):
	mkdir -p $@

$(DIST_DIR)/index.html:
	pnpm run build

.PHONY: clean
clean:
	rm -rf $(OUT_DIR)
	rm -rf $(DIST_DIR)
