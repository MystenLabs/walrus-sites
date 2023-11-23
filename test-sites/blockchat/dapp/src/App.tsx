import { ConnectButton } from "@mysten/dapp-kit";
import { Box, Container, Flex, Heading } from "@radix-ui/themes";
import { AllMessages } from "./Messages";
import { Chat } from "./Chat";

function App() {
    return (
        <>
            <Flex
                position="sticky"
                px="4"
                py="2"
                justify="between"
                style={{
                    borderBottom: "1px solid var(--gray-a2)",
                }}
            >
                <Box>
                    <Heading>Blockchat</Heading>
                </Box>

                <Box>
                    <ConnectButton />
                </Box>
            </Flex>
            <Container py="4" px="9" pb="9">
                <AllMessages />
                <Flex justify="center">
                    {/* <Flex gap="3" px="4" py="2" justify={"center"}
            style={{
              position: "sticky",
              bottom: "60px",
            }}
          >
          </Flex> */}
                </Flex>
                {/* White space to ensure no overlap between chat bar and messages */}
                <Flex p="3" style={{
                    position: "sticky", bottom: "50px", width: "100%",
                    // border: "1px solid var(--gray-a8)",
                    borderRadius: "10px",
                    backgroundColor: "white",
                    boxShadow: "0 0 20px 0 rgba(0,0,0,0.2)",

                }} >
                    <Chat />
                </Flex>
            </Container>
        </>
    );
}

export default App;
