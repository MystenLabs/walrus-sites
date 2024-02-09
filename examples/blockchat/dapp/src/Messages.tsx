// Show the list of messages received in the chat
import { useCurrentAccount, useSuiClientQuery } from "@mysten/dapp-kit";
import { MoveStruct } from "@mysten/sui.js/client";
import { SuiObjectData } from "@mysten/sui.js/client";
import { Box, Avatar, Card, Flex, Heading, Text, Link } from "@radix-ui/themes";
import { useEffect } from "react";
 
 
export const PACKAGE_ID = "0x2003d0b7386b8fe7a6cd4f43445b8075830ae570621d31b91641218e621842c2";
export const CHAT_ID = "0x8745714410272de43e6b81917b8a30a05a90e8753d506152f44ac3937efac40b";
export const NETWORK= "testnet";

export interface Chat {
    title: string,
    messages: Message[],
}

function defaltChat(): Chat {
    return {
        title: "",
        messages: [],
    }
}

export interface Message {
    object_id: string;
    author: string;
    author_addr: string;
    text: string;
    published_at: number;
}

export function AllMessages() {
    const account = useCurrentAccount();

    const { data, isLoading, error, refetch } = useSuiClientQuery("getObject", {
        id: CHAT_ID,
        options: {
            showContent: true,
            showOwner: true,
        }
    });

    useEffect(() => {
        const intervalId = setInterval(() => {
            refetch();
        }, 500);
        return () => clearInterval(intervalId);
    }, []);

    var cur_addr = "";
    if (account) {
        cur_addr = account.address;
    }

    if (isLoading) return <Text>Loading...</Text>;
    if (error) return <Text>Error: {error.message}</Text>;
    if (!data.data) return <Text>Not found</Text>;

    let chatData = getChatData(data.data);
    if (!chatData) return <Text>Error loading messages</Text>;

    // enumerate the messages and create the views
    let chat_link = `https://suiexplorer.com/object/${CHAT_ID}?network=${NETWORK}`
    return (
        <>
            <Heading><Text color="gray">blockchat::</Text><Link href={chat_link}>{chatData.title}</Link></Heading>
            <Flex gap="3" direction="column" pb="9">
                {chatData.messages.map((m: Message, idx: number) => { return MessageView(m, idx, cur_addr); })}
            </Flex>
        </>
    )

}

export function MessageView(message: Message, idx: number, cur_addr: string = "") {
    if (!message) return null;
    // let author_initial = message.author[0];
    let ts = new Date(message.published_at * 1).toLocaleString();
    let initial = message.author[0];
    let addr_link = `https://suiexplorer.com/address/${message.author_addr}?network=${NETWORK}`;
    if (cur_addr === message.author_addr) {
        return (OwnMessageView(message, idx, ts, initial, addr_link));
    }
    return (
        <Flex gap="3" key={idx}>
            <Card>
                <Flex gap="3">
                    <Avatar
                        size="3"
                        src=""
                        radius="full"
                        // Use the first letter of the author's name as avatar
                        fallback={initial}
                    />
                    {InnerMessage(message, addr_link, ts)}
                </Flex>
            </Card>
        </Flex>
    )
}

function InnerMessage(message: Message, addr_link: string, ts: string) {
    return (
        <>
        <Box>
            <Text as="div" size="2" color="gray">
                <Link href={addr_link}>{message.author}</Link> @ {ts}
            </Text>
            <Text as="div" size="2">
                {message.text}
            </Text>
        </Box>
        </>
    )
}

function OwnMessageView(message: Message, idx: number, ts: string, initial: string, addr_link: string) {
    return (
        <Flex gap="3" key={idx} justify={"end"}>
            <Card>
                <Flex gap="3">
                    <Avatar
                        size="3"
                        src=""
                        radius="full"
                        color="green"
                        fallback={initial}
                    />
                    {InnerMessage(message, addr_link, ts)}
                </Flex>
            </Card>
        </Flex>
    )
}

function getChatData(data: SuiObjectData): Chat {
    if (data.content?.dataType !== "moveObject") {
        return defaltChat();
    }
    let flds = data.content.fields as { title: string, messages: MoveStruct[] };
    var object_id: string = data.objectId;
    let msg = flds.messages;
    let messages: Message[] = msg.map((m: MoveStruct) => {
        // HACK
        let fields = (m as any).fields;
        fields.object_id = object_id;
        return fields as Message;
    });
    let title = flds.title as string;
    return { title, messages };
}
