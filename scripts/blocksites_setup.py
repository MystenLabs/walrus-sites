"""Script to set up the blocksites proof of concept.

It publishes the smart contract, adds all the test sites including the
blockchat dApp, and updates the vercel app."""

import json
import os
import re
import subprocess
from argparse import ArgumentParser

from const import (
    BLOCKCHAT_CONTRACT,
    BLOCKCHAT_DAPP,
    BLOCKCHAT_HTML,
    BLOCKSITE_CONTRACT,
    GAS_BUDGET,
    LANDING,
    MESSAGES,
    NETWORK,
    PATHS,
    PORTAL_CONST,
    SUI_BINARY,
    SW_PORTAL_CONST,
)
from publish_big_site import compress_contents, publish_big_site


def publish_html(package_id: str, path: str) -> str:
    with open(path, "r") as infile:
        html = infile.read()
    html = compress_contents(html)
    site_id = publish_big_site(html, package_id)
    return site_id


def publish_package(path: str) -> str:
    cur_dir = os.getcwd()
    os.chdir(path)
    cmd = [
        SUI_BINARY,
        "client",
        "publish",
        "--json",
        "--skip-dependency-verification",
        "--gas-budget",
        str(GAS_BUDGET),
    ]
    output = subprocess.run(cmd, capture_output=True)
    result = json.loads(output.stdout)
    package = [
        x["packageId"] for x in result["objectChanges"] if x["type"] == "published"
    ]
    os.chdir(cur_dir)
    return package[0]


def inline_site(path):
    cmd = ["inliner", _index_file(path), ">", _inlined_file(path)]
    with open(_inlined_file(path), "w") as pipefile:
        subprocess.run(cmd, stdout=pipefile, stderr=subprocess.DEVNULL)


def _inlined_file(path: str) -> str:
    return os.path.join(path, "inlined.html")


def _index_file(path: str) -> str:
    return os.path.join(path, "index.html")


def update_features_link(landing_file: str, features_id: str):
    """Update the link to the features site in the landing site."""
    with open(landing_file, "r") as infile:
        html = infile.read()
    new_url = '<a href="/#' + features_id + '">features page!</a>'
    html = re.sub(f'<a href="\/#0x\w+">features page!<\/a>', new_url, html)
    with open(landing_file, "w") as outfile:
        outfile.write(html)


def create_chat(package_id, name):
    """Create a new blockchat chat"""
    old_dir = os.getcwd()
    os.chdir(BLOCKCHAT_CONTRACT)
    cmd = [
        SUI_BINARY,
        "client",
        "call",
        "--function",
        "create_chat",
        "--module",
        "blockchat",
        "--package",
        package_id,
        "--gas-budget",
        str(GAS_BUDGET),
        "--json",
        "--args",
        f"{name}",
    ]
    output = subprocess.run(cmd, capture_output=True)
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
        if x["objectType"].startswith(f"{package_id}::blockchat::Chat")
    ]
    os.chdir(old_dir)
    return output[0]


def update_blockchat_dapp_constants(package_id: str, chat_id: str, network: str):
    with open(MESSAGES, "r") as infile:
        data = infile.read()
    new_pkg = f'export const PACKAGE_ID = "{package_id}";'
    new_chat = f'export const CHAT_ID = "{chat_id}";'
    new_network = f'export const NETWORK= "{network}";'
    data = re.sub(f'export const PACKAGE_ID = "\w+";', new_pkg, data)
    data = re.sub(f'export const CHAT_ID = "\w+";', new_chat, data)
    data = re.sub(f'export const NETWORK = "\w+";', new_network, data)
    with open(MESSAGES, "w") as outfile:
        outfile.write(data)


def update_portal_constants(package_id: str):
    # TODO: Fix export to other networks
    new_devnet_pkg = f'export const DEVNET_PACKAGE_ID = "{package_id}";'
    with open(PORTAL_CONST, "r") as infile:
        data = infile.read()
    data = re.sub(f'export const DEVNET_PACKAGE_ID = "\w+";', new_devnet_pkg, data)
    with open(PORTAL_CONST, "w") as outfile:
        outfile.write(data)


def make_blockchat_dapp(path: str):
    """Run the makefile on the blockchat dapp to build a single index.html."""
    old_dir = os.getcwd()
    os.chdir(path)
    cmd = ["make", "clean"]
    subprocess.run(cmd, capture_output=True)
    cmd = ["make"]
    subprocess.run(cmd, capture_output=True)
    os.chdir(old_dir)


def vercel_publish_prod(path: str):
    old_dir = os.getcwd()
    os.chdir(path)
    cmd = ["pnpm", "vercel", "--prod"]
    subprocess.run(cmd, capture_output=True)
    os.chdir(old_dir)


def parse_args():
    parser = ArgumentParser()
    parser.add_argument("network", default="localnet")
    parser.add_argument("--blocksite-package", default=None, type=str)
    parser.add_argument("--blockchat-package", default=None, type=str)
    return parser.parse_args()


def sw_portal_constants(args, ids) -> str:
    constants = ""
    if args.network == "localnet":
        base_url = "localhost:8000"
    else:
        base_url = "blocksite.net"
    constants += f'export const NETWORK = "{args.network}"\n'
    constants += f'export const BASE_URL = "{base_url}"\n'
    constants += "export const SITE_NAMES: { [key: string]: string } = {\n"
    constants += f'    blockchat: "{ids["blockchat"]},"\n'
    constants += f'    snake: "{ids["snake"]}",\n'
    constants += "};"
    return constants


def main():
    args = parse_args()

    if args.blocksite_package:
        package_id = args.blocksite_package
        print(f"Reused blocksite package: {package_id}")
    else:
        package_id = publish_package(BLOCKSITE_CONTRACT)
        print(f"Published new blocksite package: {package_id}")

    # Publish all the "simple" sites
    ids = {}
    for key in ["snake", "image", "features"]:
        inline_site(PATHS[key])
        site_id = publish_html(package_id, _inlined_file(PATHS[key]))
        ids[key] = site_id
        print(f"Site published: {site_id}")

    update_features_link(_index_file(LANDING), ids["features"])
    inline_site(LANDING)
    site_id = publish_html(package_id, _inlined_file(LANDING))
    ids["landing"] = site_id
    print(f"Landing site published: {site_id}")

    # Publish the blockchat contract and site
    if args.blockchat_package:
        blockchat_package = args.blockchat_package
        print(f"Reused blockchat package: {blockchat_package}")
    else:
        blockchat_package = publish_package(BLOCKCHAT_CONTRACT)
        print(f"Published new blockchat package: {blockchat_package}")
    chat_id = create_chat(blockchat_package, "Example Chat")
    print(f"Chat created: {chat_id}")
    update_blockchat_dapp_constants(blockchat_package, chat_id, NETWORK)
    make_blockchat_dapp(BLOCKCHAT_DAPP)
    dapp_id = publish_html(package_id, _index_file(BLOCKCHAT_HTML))
    print(f"Blockchat dapp created: {dapp_id}")
    ids["blockchat"] = dapp_id

    # Update the sw portal constants
    constants = sw_portal_constants(args, ids)
    with open(SW_PORTAL_CONST, "w") as outfile:
        outfile.write(constants)

    # Update & deploy the portal
    # update_portal_constants(package_id)
    # update_portal_links(
    #    PORTAL_MAIN,
    #    ids["landing"],
    #    ids["features"],
    #    ids["image"],
    #    ids["snake"],
    #    dapp_id,
    # )
    # vercel_publish_prod(PORTAL_APP)
    # print("Portal published to vercel.")


if __name__ == "__main__":
    main()
