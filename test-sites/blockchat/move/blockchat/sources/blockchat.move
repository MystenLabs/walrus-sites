module blockchat::blockchat {
    use std::string::String;
    use std::vector::{Self};
    use sui::transfer;    
    use sui::object::{Self, UID, ID};
    use sui::tx_context::{Self, TxContext};
    use sui::clock::{Self, Clock};
    use sui::event;

    struct Chat has key {
        id: UID,
        title: String,
        messages: vector<Message>,
    }

    struct Message has key, store {
        id: UID,
        author: String,
        author_addr: address,
        text: String,
        published_at: u64,
    }

    struct MessagePublished has copy, drop {
        message_id: ID,
    }

    public fun create_chat(title: String, ctx: &mut TxContext) {
        transfer::share_object(Chat {
            id: object::new(ctx),
            title,
            messages: vector::empty(),
        });
    }

    public fun new_message(author: String, text: String, clk: &Clock, ctx: &mut TxContext): Message {
        Message {
            id: object::new(ctx),
            author,
            author_addr: tx_context::sender(ctx),
            text,
            published_at: clock::timestamp_ms(clk),
        }
    }

    // Append a message to a chat
    public fun publish(message: Message, chat: &mut Chat) {
        vector::push_back(&mut chat.messages, message);
    }

    // Helper for cli calls
    public fun new_message_and_publish(author: String, text: String, chat: &mut Chat, clk: &Clock, ctx: &mut TxContext) {
        let message = new_message(author, text, clk, ctx);
        let event = MessagePublished {
            message_id: object::id(&message),
        };
        event::emit(event);
        publish(message, chat);
    }
}