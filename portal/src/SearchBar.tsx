import { MagnifyingGlassIcon } from "@radix-ui/react-icons";
import { Heading, TextField } from "@radix-ui/themes";
import { useRef, KeyboardEvent } from 'react';
import { isValidSuiObjectId } from "@mysten/sui.js/utils";

export function SearchBar() {
    const inputRef = useRef<HTMLInputElement>(null);

    function browse() {
        const inputValue = inputRef.current?.value;
        if (inputValue) {
            if (isValidSuiObjectId(inputValue)) {
                window.location.hash = inputValue;
                window.location.reload();
            }
        }
    };

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key === 'Enter') {
            browse();
        }
    };

    return (
        <>
            <Heading size="3">Browse a blocksite (no wallet required)</Heading>
            <TextField.Root>
                <TextField.Slot>
                    <MagnifyingGlassIcon height="16" width="16" />
                </TextField.Slot>
                <TextField.Input
                    placeholder="BlockSite Object ID…"
                    ref={inputRef}
                    onKeyDown={handleKeyDown}
                />
            </TextField.Root>
        </>
    );
}