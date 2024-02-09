import { useCurrentAccount, useSignAndExecuteTransactionBlock, useSuiClient } from "@mysten/dapp-kit";
import { TextField, Flex, Box, Text, Button, Separator, Strong } from "@radix-ui/themes";
import { useState } from "react";
import { CHAT_ID, PACKAGE_ID } from "./Messages";
import { TransactionBlock } from "@mysten/sui.js/transactions";
import { SUI_CLOCK_OBJECT_ID } from "@mysten/sui.js/utils";


export function Chat() {
   
    const client = useSuiClient();
    const account = useCurrentAccount();
    const [name, setName] = useState("");
    const [message, setMessage] = useState("");
    const { mutate: signAndExecute } = useSignAndExecuteTransactionBlock();

    if (!account) return <Text>Connect your wallet to start chatting...</Text>;

    const handleChangeName = (event: any) => {
        setName(event.target.value);
    };
    const handleEnterName = (event: any) => {
        if (event.key === 'Enter') {
            console.log(name);
        }
    }
    const handleChangeMessage = (event: any) => {
        setMessage(event.target.value);
    };
    const handleEnterMessage = (event: any) => {
        if (event.key === 'Enter') {
            console.log(message);
            sendMessage();
        }
    }

    return (
        <>
            <Flex
                direction="column"
                width={"100%"}
            >
                <Flex width="100%" px="3" gap="1" justify="center" height="6" >
                    <Box width="max-content">
                        <TextField.Root>
                            <TextField.Input height="1" value={name} onChange={handleChangeName} onKeyDown={handleEnterName} style={{ border: "none" }} placeholder="Your name..." />
                        </TextField.Root>
                    </Box>
                    <Separator orientation="vertical" size="4" />
                    <Box width="100%">
                        <TextField.Root>
                            <TextField.Input height="1" value={message} onChange={handleChangeMessage} onKeyDown={handleEnterMessage} placeholder="Your message..." />
                        </TextField.Root>
                    </Box>
                    <Box>
                        <Button radius="full" onClick={() => sendMessage()}>Send</Button>
                    </Box>
                </Flex>
                <Flex justify="center" px="9">
                    <Text color="orange"><Strong>Note:</Strong> This chat is unencrypted and publicly visible on Sui!</Text>
                </Flex>
            </Flex>
        </>
    )

    function sendMessage() {
        if (!name || !message) return;
        console.log(name, message);
        // Send the message with a Sui transaction

        const txb = new TransactionBlock();
        let [messageTx] = txb.moveCall({
            arguments: [txb.pure.string(name), txb.pure.string(message), txb.object(SUI_CLOCK_OBJECT_ID)],
            target: `${PACKAGE_ID}::blockchat::new_message`
        })
        txb.moveCall({
            arguments: [messageTx, txb.object(CHAT_ID)],
            target: `${PACKAGE_ID}::blockchat::publish`
        })

        signAndExecute(
            {
                transactionBlock: txb,
                options: {
                    showEffects: true,
                    showObjectChanges: true,
                },
            },
            {
                // HACK: any?
                onSuccess: (tx: any) => {
                    // Reset the input field
                    setMessage("");
                    // Scroll to the bottom of the chat to see the new message
                    window.scrollTo(0, document.body.scrollHeight);
                    client
                        .waitForTransactionBlock({
                            digest: tx.digest,
                        })
                        .then(() => {
                            const objectId = tx.effects?.created?.[0]?.reference?.objectId;
                            if (objectId) {
                                console.log("Sent message with object id: ", objectId);
                            }
                        });
                },
            },
        )

    }


}
;