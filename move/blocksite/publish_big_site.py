import subprocess
import json
import os

SUI_BINARY = "sui"
GAS_BUDGET = 10_000_000_000

contract_dir = os.path.join(
    os.path.dirname(os.path.realpath(__file__)), "."
)

def deploy_contract():
    old_wd = os.getcwd()
    os.chdir(contract_dir)
    command = [
        SUI_BINARY,
        "client",
        "publish",
        "--skip-fetch-latest-git-deps",
        "--gas-budget",
        f"{GAS_BUDGET}",
        "--json",
    ]
    output = subprocess.run(command, capture_output=True)
    os.chdir(old_wd)
    output = json.loads(output.stdout)
    output = [
        x["packageId"] for x in output["objectChanges"] if x["type"] == "published"
    ]
    return output[0]


def create_blocksite(package_id, contents):
    command = [
        SUI_BINARY,
        "client",
        "call",
        "--function",
        "create_to_sender",
        "--module",
        "blocksite",
        "--package",
        package_id,
        "--gas-budget",
        f"{GAS_BUDGET}",
        "--json",
        "--args",
        f"{contents}",
        "0x6",
    ]
    print(command)
    output = subprocess.run(command, capture_output=True)
    print(output.stderr)
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
        if x["objectType"].startswith(f"{package_id}::blocksite::BlockSite")
    ]
    return output[0]

def add_piece(site_id, piece, package_id):
    command = [
        SUI_BINARY,
        "client",
        "call",
        "--function",
        "add_piece",
        "--module",
        "blocksite",
        "--package",
        package_id,
        "--gas-budget",
        f"{GAS_BUDGET}",
        "--json",
        "--args",
        f"{site_id}",
        f"{piece}",
        "0x6",
    ]
    output = subprocess.run(command, capture_output=True)
    print(
    output.stderr
    )
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
    ]
    return output[0]


def publish_big_site(site, package_id):
    PIECE_SIZE = 10_000
    site_id = create_blocksite(package_id, site[:PIECE_SIZE])
    print(f"Chat ID: {site_id}")
    for idx in range(PIECE_SIZE, len(site), PIECE_SIZE):
        add_piece( site_id, f"{site[idx:idx + PIECE_SIZE]}", package_id)
        print(f"piece added: {idx}")


def main():
    import sys
    sitefile = sys.argv[1]
    with open(sitefile) as f:
        site = f.read()

    # Convert the base64 string to bytes
    # TODO: back and forth conversion to base 64
    import base64
    site = base64.b64decode(site)
    site = [int(x) for x in site]
    print(len(site))
    package_id = deploy_contract()
    print(f"Package ID: {package_id}")
    publish_big_site(site, package_id)



if __name__ == "__main__":
    main()