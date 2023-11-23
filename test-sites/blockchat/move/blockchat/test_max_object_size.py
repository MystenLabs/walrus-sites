"""Keep adding messages to the chat until we hit the object size limits."""

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
        "--gas-budget",
        f"{GAS_BUDGET}",
        "--json",
    ]
    output = subprocess.run(command, capture_output=True)
    os.chdir(old_wd)
    print(output)
    output = json.loads(output.stdout)
    output = [
        x["packageId"] for x in output["objectChanges"] if x["type"] == "published"
    ]
    return output[0]


def create_chat(package_id, name):
    command = [
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
        f"{GAS_BUDGET}",
        "--json",
        "--args",
        f"{name}",
    ]
    output = subprocess.run(command, capture_output=True)
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
        if x["objectType"].startswith(f"{package_id}::blockchat::Chat")
    ]
    return output[0]

def new_message_and_publish(author, text, chat_id, package_id):
    command = [
        SUI_BINARY,
        "client",
        "call",
        "--function",
        "new_message_and_publish",
        "--module",
        "blockchat",
        "--package",
        package_id,
        "--gas-budget",
        f"{GAS_BUDGET}",
        "--json",
        "--args",
        f"{author}",
        f"{text}",
        f"{chat_id}",
        f"0x6",
    ]
    output = subprocess.run(command, capture_output=True)
    print(output)
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
    ]
    return output[0]

# def massive_message(author, text, chat_id, package_id)


def main():
    package_id = deploy_contract()
    print(f"Package ID: {package_id}")

    chat_id = create_chat(package_id, "My little dummy chat")
    print(f"Chat ID: {chat_id}")

    N_BYTES = 15_000
    BYTES = "0x" + "0" * N_BYTES
    idx = 0
    while True:
        idx += 1
        new_message_and_publish("Giac", BYTES, chat_id, package_id)
        print(f"Message {idx} sent, for a total of (at least) {idx * N_BYTES} bytes.")

if __name__ == "__main__":
    main()