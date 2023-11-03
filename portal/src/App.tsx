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
                                    <li>A a description of the blocksites project: <Link href="/#0x93e9e43be38372f7915aac3bdd30e5c2f2d22c699475e5944f06d8fb67b6874c"><code>0x93e9e43be38372f7915aac3bdd30e5c2f2d22c699475e5944f06d8fb67b6874c</code></Link></li>
                                    <li>The current feature list supported by blocksites: <Link href="/#0x9bfd168a1f3efe281dab315a552249c6e08b01d89fda7cd8b89a28bb68b7d644"><code>0x9bfd168a1f3efe281dab315a552249c6e08b01d89fda7cd8b89a28bb68b7d644</code></Link></li>
                                    <li>A simple blocksite with images and javascript: <Link href="/#0x83eb6879ed4ef76cced88c70c806b982e315afc2ba23b8afd189ff098aec4080"><code>0x83eb6879ed4ef76cced88c70c806b982e315afc2ba23b8afd189ff098aec4080</code></Link></li>
                                    <li>Play snake on a blocksite: <Link href="/#0x50182427c57fb6c050bcbb3755bde94a99749da4507242f2402d31549a2fd12d"><code>0x50182427c57fb6c050bcbb3755bde94a99749da4507242f2402d31549a2fd12d</code></Link></li>
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
