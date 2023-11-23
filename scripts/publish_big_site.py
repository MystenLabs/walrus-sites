import subprocess
import json
import os
import zlib
from tqdm import tqdm
from const import BLOCKSITE_CONTRACT, SUI_BINARY, GAS_BUDGET


def create_blocksite(package_id, contents):
    old_dir = os.getcwd()
    os.chdir(BLOCKSITE_CONTRACT)
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
    output = subprocess.run(command, capture_output=True)
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
        if x["objectType"].startswith(f"{package_id}::blocksite::BlockSite")
    ]
    os.chdir(old_dir)
    return output[0]


def add_piece(site_id, piece, package_id):
    old_dir = os.getcwd()
    os.chdir(BLOCKSITE_CONTRACT)
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
    output = json.loads(output.stdout)
    output = [x["objectId"] for x in output["objectChanges"]]
    os.chdir(old_dir)
    return output[0]


def publish_big_site(site, package_id):
    PIECE_SIZE = 15_000
    print(f"Site length to publish: {len(site)}")
    site_id = create_blocksite(package_id, site[:PIECE_SIZE])
    for idx in tqdm(
        range(PIECE_SIZE, len(site), PIECE_SIZE), desc="Publishing site pieces"
    ):
        add_piece(site_id, f"{site[idx:idx + PIECE_SIZE]}", package_id)
    return site_id


def compress_contents(contents):
    compressed = zlib.compress(str.encode(contents))
    # HACK: Return the bytes as a list of integers for the cli
    return [int(x) for x in compressed]


# def main():
#    import sys
#    sitefile = sys.argv[1]
#    with open(sitefile) as f:
#        site = f.read()
#
#    # Convert the base64 string to bytes
#    # TODO: back and forth conversion to base 64
#    import base64
#    site = base64.b64decode(site)
#    site = [int(x) for x in site]
#    print(f"Package ID: {package_id}")
#    publish_big_site(site, package_idej
#
#
# if __name__ == "__main__":
#    main()
