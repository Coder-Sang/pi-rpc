//! Run the complete wire suite independently for every enabled protocol.
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! protocol_suite {
    ($module:ident, $profile:ident) => {
        mod $module {
            use pi_rpc::protocol::$module::{Event, KnownEvent};
            use pi_rpc::{Error, protocol::*};
            use serde_json::{Value, json};
            type Record = pi_rpc::protocol::Record<pi_rpc::$profile>;
            fn decode_record(value: Value) -> pi_rpc::Result<Record> {
                pi_rpc::protocol::decode_record::<pi_rpc::$profile>(value)
            }
            include!("support/protocol_fixtures.rs");
            const STACK_TEST: &str = concat!(
                stringify!($module),
                "::stack::boundary_parse_convert_clone_serialize_and_drop_on_two_mib_stack"
            );
            mod stack {
                include!("protocol/stack.rs");
            }
            mod command {
                include!("protocol/command.rs");
            }
            mod event {
                include!("protocol/event.rs");
            }
            mod message {
                include!("protocol/message.rs");
            }
            mod response {
                include!("protocol/response.rs");
            }
            mod ui {
                include!("protocol/ui.rs");
            }
            mod response_negative {
                include!("protocol/response_negative.rs");
            }
        }
    };
}
#[cfg(feature = "pi_1_3")]
protocol_suite!(v1_3, Pi13);
#[cfg(feature = "pi_1_4")]
protocol_suite!(v1_4, Pi14);
#[cfg(feature = "pi_1_5")]
protocol_suite!(v1_5, Pi15);
