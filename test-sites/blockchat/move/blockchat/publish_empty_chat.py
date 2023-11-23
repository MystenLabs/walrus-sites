"""Create a dummy chat with a few messages."""

from argparse import ArgumentParser
import subprocess
import json
import os

SUI_BINARY = "sui"

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
        "3000000000",
        "--json",
    ]
    output = subprocess.run(command, capture_output=True)
    os.chdir(old_wd)
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
        "30000000",
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
        "30000000",
        "--json",
        "--args",
        f"{author}",
        f"{text}",
        f"{chat_id}",
        f"0x6",
    ]
    output = subprocess.run(command, capture_output=True)
    output = json.loads(output.stdout)
    output = [
        x["objectId"]
        for x in output["objectChanges"]
    ]
    return output[0]


def parse_args():
    parser = ArgumentParser()
    parser.add_argument("-p", "--package-id", type=str, required=False)
    parser.add_argument("-n", "--chat-name", type=str, default="Demo Chat!")
    return parser.parse_args()

def main():
    args = parse_args()
    if not args.package_id:
        print("Deploying contract")
        package_id = deploy_contract()
        print(f"Package ID: {package_id}")
    else: 
        package_id = args.package_id

    chat_id = create_chat(package_id, args.chat_name)
    print(f"Chat ID: {chat_id}")

    # message_id_1 = new_message_and_publish("Giac", "Hello", chat_id, package_id)
    # message_id_2 = new_message_and_publish("JP", "Hi", chat_id, package_id)
    # message_id_3 = new_message_and_publish("Karl", "How are you?", chat_id, package_id)
    # message_id_4 = new_message_and_publish("Markus", "Bye", chat_id, package_id)

if __name__ == "__main__":
    main()