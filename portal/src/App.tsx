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
                                    <li>A a description of the blocksites project: <Link href="/#0xf1e1352f393ed3d7077107fc5df82f3e3ce56c6baf6463923bb1b445cef10397"><code>0xf1e1352f393ed3d7077107fc5df82f3e3ce56c6baf6463923bb1b445cef10397</code></Link></li>
                                    <li>The current feature list supported by blocksites: <Link href="/#0xd8353df3e48a3efdc9b45fd5f42549e5be037aba077d45e189ed3b9e2c85b089"><code>0xd8353df3e48a3efdc9b45fd5f42549e5be037aba077d45e189ed3b9e2c85b089</code></Link></li>
                                    <li>A simple blocksite with images and javascript: <Link href="/#0x817077c2e1e5ed4d45535166daf2ebb187e1c988fc387c0c5db6f6035358d0ea"><code>0x817077c2e1e5ed4d45535166daf2ebb187e1c988fc387c0c5db6f6035358d0ea</code></Link></li>
                                    <li>Play snake on a blocksite: <Link href="/#0x3daaa5594b73b7bab3f4c7da2d172f0631dd64765cd584a21ae4985ca6f93f85"><code>0x3daaa5594b73b7bab3f4c7da2d172f0631dd64765cd584a21ae4985ca6f93f85</code></Link></li>
                                    <li>(with hacked wallet only: Try a dApp): <Link href="/#0x61a02cb006b131964e7202cb3c3bcdc19700eee2ea2b082e214e5569bb7c0d93"><code>0x61a02cb006b131964e7202cb3c3bcdc19700eee2ea2b082e214e5569bb7c0d93</code></Link></li>
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
