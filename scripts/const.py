import os

SUI_BINARY = "sui"
GAS_BUDGET = 500_000_000
NETWORK = "devnet"

BLOCKSITE_CONTRACT = os.path.normpath("../move/blocksite")
PORTAL_APP = os.path.normpath("../portal")
PORTAL_MAIN = os.path.join(PORTAL_APP, "src/App.tsx")
PORTAL_CONST = os.path.join(PORTAL_APP, "src/constants.ts")
TEST_SITES_DIR = os.path.normpath("../test-sites")
BLOCKCHAT_CONTRACT = os.path.join(TEST_SITES_DIR, "blockchat/move/blockchat")
BLOCKCHAT_DAPP = os.path.join(TEST_SITES_DIR, "blockchat/dapp")
BLOCKCHAT_HTML = os.path.join(BLOCKCHAT_DAPP, "single-html")
MESSAGES = os.path.join(BLOCKCHAT_DAPP, "src/Messages.tsx")
LANDING = os.path.join(TEST_SITES_DIR, "landing-blocksite")
SNAKE = os.path.join(TEST_SITES_DIR, "snake-blocksite")
FEATURES = os.path.join(TEST_SITES_DIR, "features-blocksite")
IMAGE = os.path.join(TEST_SITES_DIR, "image-blocksite")

PATHS = {"snake": SNAKE, "image": IMAGE, "features": FEATURES, "landing": LANDING}
