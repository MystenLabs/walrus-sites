import { ConnectButton, useCurrentAccount } from "@mysten/dapp-kit";
import { isValidSuiObjectId } from "@mysten/sui.js/utils";
import { Box, Text, Container, Flex, Heading, Link } from "@radix-ui/themes";
import { useState } from "react";
import { CreateBlocksite } from "./CreateBlocksite";
import { BrowseSite } from "./BrowseSite";
import { SearchBar } from "./SearchBar";

function App() {
    const currentAccount = useCurrentAccount();
    const [counterId, setCounter] = useState(() => {
        const hash = window.location.hash.slice(1);
        return isValidSuiObjectId(hash) ? hash : null;
    });

    return (
        <>
            {
                counterId ? (
                    <BrowseSite id={counterId} />
                ) : (
                    <>
                        <Flex
                            position="sticky"
                            px="4"
                            py="2"
                            justify="between"
                            style={{
                                borderBottom: "0px solid var(--gray-a2)",
                            }}
                        >
                            <Flex>
                            <Box>
                                <Link href="/">
                                    <Heading>BlockSite </Heading>
                                </Link>
                            </Box>
                            <Box>
                            <Text>(devnet)</Text>
                            </Box>
                            </Flex>
                            <Box>
                                <ConnectButton />
                            </Box>
                        </Flex >
                        <Container>
                            {
                                currentAccount ? (
                                    <>
                                        <SearchBar />
                                        <CreateBlocksite
                                            onCreated={(id) => {
                                                window.location.hash = id;
                                                setCounter(id);
                                            }}
                                        />
                                    </>
                                ) : (
                                    <>
                                        <SearchBar />
                                        <Heading size="3">... or connect a wallet to create a blocksite</Heading>
                                    </>
                                )
                            }
                        </Container>
                        <Container pt="9">
                            <Heading size="3">If you don't know where to start, check out:</Heading>
                            <Text>
                                <ul>
                                    <li>A a description of the blocksites project: <Link href="/#0x6e95fa8fff2147583f42d54ed4352505e8556b6fd5e27a75f354cee910182bc8"><code>0x6e95fa8fff2147583f42d54ed4352505e8556b6fd5e27a75f354cee910182bc8</code></Link></li>
                                    <li>The current feature list supported by blocksites: <Link href="/#0x491effc375c2cb94fbb9459fb3185b5c0800c52c9252d2d045f6dfad89fb8487"><code>0x491effc375c2cb94fbb9459fb3185b5c0800c52c9252d2d045f6dfad89fb8487</code></Link></li>
                                    <li>A simple blocksite with images and javascript: <Link href="/#0x29d5206be278c74923743b9e7284346cdcd4d3632534053123114d8af8714a21"><code>0x29d5206be278c74923743b9e7284346cdcd4d3632534053123114d8af8714a21</code></Link></li>
                                    <li>Play snake on a blocksite: <Link href="/#0x95d9b1c1475fca7884299eb695b2deb67f52fe7815a10cd1596e4e4fb6cd6c68"><code>0x95d9b1c1475fca7884299eb695b2deb67f52fe7815a10cd1596e4e4fb6cd6c68</code></Link></li>
                                </ul>
                            </Text>
                        </Container>
                    </>
                )
            }
        </>
    )
}

export default App;
