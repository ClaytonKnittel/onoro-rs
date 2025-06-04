#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
mod error {
    pub enum Error {
        ProtoDecode(String),
    }
}
mod file_server {
    use std::{env, net::{Ipv6Addr, SocketAddrV6}};
    use tokio::task::JoinHandle;
    use warp::Filter;
    pub fn create_static_file_server() -> JoinHandle<()> {
        tokio::spawn(async {
            tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();
            let log_filter = warp::trace::request();
            let route = warp::fs::dir(
                    env::current_dir()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .join("web/dist/dev/static"),
                )
                .with(log_filter);
            warp::serve(route)
                .run(
                    SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 2001, 0, 0),
                )
                .await;
        })
    }
}
mod initialize {
    use crate::file_server::create_static_file_server;
    use crate::socket_init::create_socket_endpoint;
    pub async fn init() {
        match {
            use ::tokio::macros::support::{maybe_done, poll_fn, Future, Pin};
            use ::tokio::macros::support::Poll::{Ready, Pending};
            let mut futures = (
                maybe_done(create_static_file_server()),
                maybe_done(create_socket_endpoint()),
            );
            let mut futures = &mut futures;
            let mut skip_next_time: u32 = 0;
            poll_fn(move |cx| {
                    const COUNT: u32 = 0 + 1 + 1;
                    let mut is_pending = false;
                    let mut to_run = COUNT;
                    let mut skip = skip_next_time;
                    skip_next_time = if skip + 1 == COUNT { 0 } else { skip + 1 };
                    loop {
                        if skip == 0 {
                            if to_run == 0 {
                                break;
                            }
                            to_run -= 1;
                            let (fut, ..) = &mut *futures;
                            let mut fut = unsafe { Pin::new_unchecked(fut) };
                            if fut.poll(cx).is_pending() {
                                is_pending = true;
                            }
                        } else {
                            skip -= 1;
                        }
                        if skip == 0 {
                            if to_run == 0 {
                                break;
                            }
                            to_run -= 1;
                            let (_, fut, ..) = &mut *futures;
                            let mut fut = unsafe { Pin::new_unchecked(fut) };
                            if fut.poll(cx).is_pending() {
                                is_pending = true;
                            }
                        } else {
                            skip -= 1;
                        }
                    }
                    if is_pending {
                        Pending
                    } else {
                        Ready((
                            {
                                let (fut, ..) = &mut futures;
                                let mut fut = unsafe { Pin::new_unchecked(fut) };
                                fut.take_output().expect("expected completed future")
                            },
                            {
                                let (_, fut, ..) = &mut futures;
                                let mut fut = unsafe { Pin::new_unchecked(fut) };
                                fut.take_output().expect("expected completed future")
                            },
                        ))
                    }
                })
                .await
        } {
            (Err(err), _) => {
                {
                    ::std::io::_print(
                        format_args!("Error joining static file server: {0:?}\n", err),
                    );
                };
            }
            (_, Err(err)) => {
                {
                    ::std::io::_print(
                        format_args!("Error joining socket server: {0:?}\n", err),
                    );
                };
            }
            (Ok(()), Ok(())) => {}
        }
    }
}
mod proto {
    use async_sockets::Status;
    use bytes::BytesMut;
    use itertools::interleave;
    use onoro::{Move, Onoro, PackedIdx, Pawn, PawnColor};
    use prost::Message;
    use serde::{
        de::{self, Visitor},
        ser, Deserialize, Deserializer, Serialize, Serializer,
    };
    use crate::error::Error;
    mod proto_impl {
        #[allow(clippy::derive_partial_eq_without_eq)]
        pub struct GameState {
            /// A list of all the pawns that have been played, along with the coordinates
            /// of each pawn. The absolute position of the pawns does not matter, only the
            /// distances between each pawn.
            #[prost(message, repeated, tag = "4")]
            pub pawns: ::prost::alloc::vec::Vec<game_state::Pawn>,
            /// If true, it is the black player's turn, otherwise the white player's turn.
            #[prost(bool, optional, tag = "1")]
            pub black_turn: ::core::option::Option<bool>,
            /// The current turn number, starting from 0.
            #[prost(uint32, optional, tag = "2")]
            pub turn_num: ::core::option::Option<u32>,
            /// True if the game is finished, meaning someone has won.
            #[prost(bool, optional, tag = "3")]
            pub finished: ::core::option::Option<bool>,
        }
        #[automatically_derived]
        #[allow(clippy::derive_partial_eq_without_eq)]
        impl ::core::clone::Clone for GameState {
            #[inline]
            fn clone(&self) -> GameState {
                GameState {
                    pawns: ::core::clone::Clone::clone(&self.pawns),
                    black_turn: ::core::clone::Clone::clone(&self.black_turn),
                    turn_num: ::core::clone::Clone::clone(&self.turn_num),
                    finished: ::core::clone::Clone::clone(&self.finished),
                }
            }
        }
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for GameState {}
        #[automatically_derived]
        #[allow(clippy::derive_partial_eq_without_eq)]
        impl ::core::cmp::PartialEq for GameState {
            #[inline]
            fn eq(&self, other: &GameState) -> bool {
                self.pawns == other.pawns && self.black_turn == other.black_turn
                    && self.turn_num == other.turn_num && self.finished == other.finished
            }
        }
        impl ::prost::Message for GameState {
            #[allow(unused_variables)]
            fn encode_raw<B>(&self, buf: &mut B)
            where
                B: ::prost::bytes::BufMut,
            {
                if let ::core::option::Option::Some(ref value) = self.black_turn {
                    ::prost::encoding::bool::encode(1u32, value, buf);
                }
                if let ::core::option::Option::Some(ref value) = self.turn_num {
                    ::prost::encoding::uint32::encode(2u32, value, buf);
                }
                if let ::core::option::Option::Some(ref value) = self.finished {
                    ::prost::encoding::bool::encode(3u32, value, buf);
                }
                for msg in &self.pawns {
                    ::prost::encoding::message::encode(4u32, msg, buf);
                }
            }
            #[allow(unused_variables)]
            fn merge_field<B>(
                &mut self,
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                buf: &mut B,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError>
            where
                B: ::prost::bytes::Buf,
            {
                const STRUCT_NAME: &'static str = "GameState";
                match tag {
                    1u32 => {
                        let mut value = &mut self.black_turn;
                        ::prost::encoding::bool::merge(
                                wire_type,
                                value.get_or_insert_with(::core::default::Default::default),
                                buf,
                                ctx,
                            )
                            .map_err(|mut error| {
                                error.push(STRUCT_NAME, "black_turn");
                                error
                            })
                    }
                    2u32 => {
                        let mut value = &mut self.turn_num;
                        ::prost::encoding::uint32::merge(
                                wire_type,
                                value.get_or_insert_with(::core::default::Default::default),
                                buf,
                                ctx,
                            )
                            .map_err(|mut error| {
                                error.push(STRUCT_NAME, "turn_num");
                                error
                            })
                    }
                    3u32 => {
                        let mut value = &mut self.finished;
                        ::prost::encoding::bool::merge(
                                wire_type,
                                value.get_or_insert_with(::core::default::Default::default),
                                buf,
                                ctx,
                            )
                            .map_err(|mut error| {
                                error.push(STRUCT_NAME, "finished");
                                error
                            })
                    }
                    4u32 => {
                        let mut value = &mut self.pawns;
                        ::prost::encoding::message::merge_repeated(
                                wire_type,
                                value,
                                buf,
                                ctx,
                            )
                            .map_err(|mut error| {
                                error.push(STRUCT_NAME, "pawns");
                                error
                            })
                    }
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
            #[inline]
            fn encoded_len(&self) -> usize {
                0
                    + self
                        .black_turn
                        .as_ref()
                        .map_or(
                            0,
                            |value| ::prost::encoding::bool::encoded_len(1u32, value),
                        )
                    + self
                        .turn_num
                        .as_ref()
                        .map_or(
                            0,
                            |value| ::prost::encoding::uint32::encoded_len(2u32, value),
                        )
                    + self
                        .finished
                        .as_ref()
                        .map_or(
                            0,
                            |value| ::prost::encoding::bool::encoded_len(3u32, value),
                        )
                    + ::prost::encoding::message::encoded_len_repeated(4u32, &self.pawns)
            }
            fn clear(&mut self) {
                self.black_turn = ::core::option::Option::None;
                self.turn_num = ::core::option::Option::None;
                self.finished = ::core::option::Option::None;
                self.pawns.clear();
            }
        }
        impl ::core::default::Default for GameState {
            fn default() -> Self {
                GameState {
                    black_turn: ::core::option::Option::None,
                    turn_num: ::core::option::Option::None,
                    finished: ::core::option::Option::None,
                    pawns: ::core::default::Default::default(),
                }
            }
        }
        impl ::core::fmt::Debug for GameState {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let mut builder = f.debug_struct("GameState");
                let builder = {
                    let wrapper = &self.pawns;
                    builder.field("pawns", &wrapper)
                };
                let builder = {
                    let wrapper = {
                        struct ScalarWrapper<'a>(&'a ::core::option::Option<bool>);
                        impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                            fn fmt(
                                &self,
                                f: &mut ::core::fmt::Formatter,
                            ) -> ::core::fmt::Result {
                                #[allow(non_snake_case)]
                                fn Inner<T>(v: T) -> T {
                                    v
                                }
                                ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                            }
                        }
                        ScalarWrapper(&self.black_turn)
                    };
                    builder.field("black_turn", &wrapper)
                };
                let builder = {
                    let wrapper = {
                        struct ScalarWrapper<'a>(&'a ::core::option::Option<u32>);
                        impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                            fn fmt(
                                &self,
                                f: &mut ::core::fmt::Formatter,
                            ) -> ::core::fmt::Result {
                                #[allow(non_snake_case)]
                                fn Inner<T>(v: T) -> T {
                                    v
                                }
                                ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                            }
                        }
                        ScalarWrapper(&self.turn_num)
                    };
                    builder.field("turn_num", &wrapper)
                };
                let builder = {
                    let wrapper = {
                        struct ScalarWrapper<'a>(&'a ::core::option::Option<bool>);
                        impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                            fn fmt(
                                &self,
                                f: &mut ::core::fmt::Formatter,
                            ) -> ::core::fmt::Result {
                                #[allow(non_snake_case)]
                                fn Inner<T>(v: T) -> T {
                                    v
                                }
                                ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                            }
                        }
                        ScalarWrapper(&self.finished)
                    };
                    builder.field("finished", &wrapper)
                };
                builder.finish()
            }
        }
        #[allow(dead_code)]
        impl GameState {
            ///Returns the value of `black_turn`, or the default value if `black_turn` is unset.
            pub fn black_turn(&self) -> bool {
                match self.black_turn {
                    ::core::option::Option::Some(val) => val,
                    ::core::option::Option::None => false,
                }
            }
            ///Returns the value of `turn_num`, or the default value if `turn_num` is unset.
            pub fn turn_num(&self) -> u32 {
                match self.turn_num {
                    ::core::option::Option::Some(val) => val,
                    ::core::option::Option::None => 0u32,
                }
            }
            ///Returns the value of `finished`, or the default value if `finished` is unset.
            pub fn finished(&self) -> bool {
                match self.finished {
                    ::core::option::Option::Some(val) => val,
                    ::core::option::Option::None => false,
                }
            }
        }
        /// Nested message and enum types in `GameState`.
        pub mod game_state {
            #[allow(clippy::derive_partial_eq_without_eq)]
            pub struct Pawn {
                /// x-coordinate of the pawn.
                #[prost(int32, optional, tag = "1")]
                pub x: ::core::option::Option<i32>,
                /// y-coordinate of the pawn, where the y-axis is 120 degrees
                /// counter-clockwise from the x-axis.
                #[prost(int32, optional, tag = "2")]
                pub y: ::core::option::Option<i32>,
                /// If true, this is a black pawn, otherwise it's a white pawn.
                #[prost(bool, optional, tag = "3")]
                pub black: ::core::option::Option<bool>,
            }
            #[automatically_derived]
            #[allow(clippy::derive_partial_eq_without_eq)]
            impl ::core::clone::Clone for Pawn {
                #[inline]
                fn clone(&self) -> Pawn {
                    Pawn {
                        x: ::core::clone::Clone::clone(&self.x),
                        y: ::core::clone::Clone::clone(&self.y),
                        black: ::core::clone::Clone::clone(&self.black),
                    }
                }
            }
            #[allow(clippy::derive_partial_eq_without_eq)]
            #[automatically_derived]
            impl ::core::marker::StructuralPartialEq for Pawn {}
            #[automatically_derived]
            #[allow(clippy::derive_partial_eq_without_eq)]
            impl ::core::cmp::PartialEq for Pawn {
                #[inline]
                fn eq(&self, other: &Pawn) -> bool {
                    self.x == other.x && self.y == other.y && self.black == other.black
                }
            }
            impl ::prost::Message for Pawn {
                #[allow(unused_variables)]
                fn encode_raw<B>(&self, buf: &mut B)
                where
                    B: ::prost::bytes::BufMut,
                {
                    if let ::core::option::Option::Some(ref value) = self.x {
                        ::prost::encoding::int32::encode(1u32, value, buf);
                    }
                    if let ::core::option::Option::Some(ref value) = self.y {
                        ::prost::encoding::int32::encode(2u32, value, buf);
                    }
                    if let ::core::option::Option::Some(ref value) = self.black {
                        ::prost::encoding::bool::encode(3u32, value, buf);
                    }
                }
                #[allow(unused_variables)]
                fn merge_field<B>(
                    &mut self,
                    tag: u32,
                    wire_type: ::prost::encoding::WireType,
                    buf: &mut B,
                    ctx: ::prost::encoding::DecodeContext,
                ) -> ::core::result::Result<(), ::prost::DecodeError>
                where
                    B: ::prost::bytes::Buf,
                {
                    const STRUCT_NAME: &'static str = "Pawn";
                    match tag {
                        1u32 => {
                            let mut value = &mut self.x;
                            ::prost::encoding::int32::merge(
                                    wire_type,
                                    value.get_or_insert_with(::core::default::Default::default),
                                    buf,
                                    ctx,
                                )
                                .map_err(|mut error| {
                                    error.push(STRUCT_NAME, "x");
                                    error
                                })
                        }
                        2u32 => {
                            let mut value = &mut self.y;
                            ::prost::encoding::int32::merge(
                                    wire_type,
                                    value.get_or_insert_with(::core::default::Default::default),
                                    buf,
                                    ctx,
                                )
                                .map_err(|mut error| {
                                    error.push(STRUCT_NAME, "y");
                                    error
                                })
                        }
                        3u32 => {
                            let mut value = &mut self.black;
                            ::prost::encoding::bool::merge(
                                    wire_type,
                                    value.get_or_insert_with(::core::default::Default::default),
                                    buf,
                                    ctx,
                                )
                                .map_err(|mut error| {
                                    error.push(STRUCT_NAME, "black");
                                    error
                                })
                        }
                        _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                    }
                }
                #[inline]
                fn encoded_len(&self) -> usize {
                    0
                        + self
                            .x
                            .as_ref()
                            .map_or(
                                0,
                                |value| ::prost::encoding::int32::encoded_len(1u32, value),
                            )
                        + self
                            .y
                            .as_ref()
                            .map_or(
                                0,
                                |value| ::prost::encoding::int32::encoded_len(2u32, value),
                            )
                        + self
                            .black
                            .as_ref()
                            .map_or(
                                0,
                                |value| ::prost::encoding::bool::encoded_len(3u32, value),
                            )
                }
                fn clear(&mut self) {
                    self.x = ::core::option::Option::None;
                    self.y = ::core::option::Option::None;
                    self.black = ::core::option::Option::None;
                }
            }
            impl ::core::default::Default for Pawn {
                fn default() -> Self {
                    Pawn {
                        x: ::core::option::Option::None,
                        y: ::core::option::Option::None,
                        black: ::core::option::Option::None,
                    }
                }
            }
            impl ::core::fmt::Debug for Pawn {
                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    let mut builder = f.debug_struct("Pawn");
                    let builder = {
                        let wrapper = {
                            struct ScalarWrapper<'a>(&'a ::core::option::Option<i32>);
                            impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                                fn fmt(
                                    &self,
                                    f: &mut ::core::fmt::Formatter,
                                ) -> ::core::fmt::Result {
                                    #[allow(non_snake_case)]
                                    fn Inner<T>(v: T) -> T {
                                        v
                                    }
                                    ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                                }
                            }
                            ScalarWrapper(&self.x)
                        };
                        builder.field("x", &wrapper)
                    };
                    let builder = {
                        let wrapper = {
                            struct ScalarWrapper<'a>(&'a ::core::option::Option<i32>);
                            impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                                fn fmt(
                                    &self,
                                    f: &mut ::core::fmt::Formatter,
                                ) -> ::core::fmt::Result {
                                    #[allow(non_snake_case)]
                                    fn Inner<T>(v: T) -> T {
                                        v
                                    }
                                    ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                                }
                            }
                            ScalarWrapper(&self.y)
                        };
                        builder.field("y", &wrapper)
                    };
                    let builder = {
                        let wrapper = {
                            struct ScalarWrapper<'a>(&'a ::core::option::Option<bool>);
                            impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                                fn fmt(
                                    &self,
                                    f: &mut ::core::fmt::Formatter,
                                ) -> ::core::fmt::Result {
                                    #[allow(non_snake_case)]
                                    fn Inner<T>(v: T) -> T {
                                        v
                                    }
                                    ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                                }
                            }
                            ScalarWrapper(&self.black)
                        };
                        builder.field("black", &wrapper)
                    };
                    builder.finish()
                }
            }
            #[allow(dead_code)]
            impl Pawn {
                ///Returns the value of `x`, or the default value if `x` is unset.
                pub fn x(&self) -> i32 {
                    match self.x {
                        ::core::option::Option::Some(val) => val,
                        ::core::option::Option::None => 0i32,
                    }
                }
                ///Returns the value of `y`, or the default value if `y` is unset.
                pub fn y(&self) -> i32 {
                    match self.y {
                        ::core::option::Option::Some(val) => val,
                        ::core::option::Option::None => 0i32,
                    }
                }
                ///Returns the value of `black`, or the default value if `black` is unset.
                pub fn black(&self) -> bool {
                    match self.black {
                        ::core::option::Option::Some(val) => val,
                        ::core::option::Option::None => false,
                    }
                }
            }
        }
        #[allow(clippy::derive_partial_eq_without_eq)]
        pub struct GameStates {
            #[prost(message, repeated, tag = "1")]
            pub state: ::prost::alloc::vec::Vec<GameState>,
        }
        #[automatically_derived]
        #[allow(clippy::derive_partial_eq_without_eq)]
        impl ::core::clone::Clone for GameStates {
            #[inline]
            fn clone(&self) -> GameStates {
                GameStates {
                    state: ::core::clone::Clone::clone(&self.state),
                }
            }
        }
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for GameStates {}
        #[automatically_derived]
        #[allow(clippy::derive_partial_eq_without_eq)]
        impl ::core::cmp::PartialEq for GameStates {
            #[inline]
            fn eq(&self, other: &GameStates) -> bool {
                self.state == other.state
            }
        }
        impl ::prost::Message for GameStates {
            #[allow(unused_variables)]
            fn encode_raw<B>(&self, buf: &mut B)
            where
                B: ::prost::bytes::BufMut,
            {
                for msg in &self.state {
                    ::prost::encoding::message::encode(1u32, msg, buf);
                }
            }
            #[allow(unused_variables)]
            fn merge_field<B>(
                &mut self,
                tag: u32,
                wire_type: ::prost::encoding::WireType,
                buf: &mut B,
                ctx: ::prost::encoding::DecodeContext,
            ) -> ::core::result::Result<(), ::prost::DecodeError>
            where
                B: ::prost::bytes::Buf,
            {
                const STRUCT_NAME: &'static str = "GameStates";
                match tag {
                    1u32 => {
                        let mut value = &mut self.state;
                        ::prost::encoding::message::merge_repeated(
                                wire_type,
                                value,
                                buf,
                                ctx,
                            )
                            .map_err(|mut error| {
                                error.push(STRUCT_NAME, "state");
                                error
                            })
                    }
                    _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            }
            #[inline]
            fn encoded_len(&self) -> usize {
                0 + ::prost::encoding::message::encoded_len_repeated(1u32, &self.state)
            }
            fn clear(&mut self) {
                self.state.clear();
            }
        }
        impl ::core::default::Default for GameStates {
            fn default() -> Self {
                GameStates {
                    state: ::core::default::Default::default(),
                }
            }
        }
        impl ::core::fmt::Debug for GameStates {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let mut builder = f.debug_struct("GameStates");
                let builder = {
                    let wrapper = &self.state;
                    builder.field("state", &wrapper)
                };
                builder.finish()
            }
        }
    }
    struct BytesMutVisitor;
    #[automatically_derived]
    impl ::core::fmt::Debug for BytesMutVisitor {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "BytesMutVisitor")
        }
    }
    impl<'de> Visitor<'de> for BytesMutVisitor {
        type Value = BytesMut;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("Expecting bytes")
        }
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(BytesMut::from(v))
        }
    }
    pub struct GameStateProto {
        game_state: proto_impl::GameState,
    }
    impl GameStateProto {
        pub fn from_onoro<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
            onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
        ) -> Self {
            Self {
                game_state: proto_impl::GameState {
                    pawns: onoro
                        .pawns()
                        .map(|pawn| proto_impl::game_state::Pawn {
                            x: Some(pawn.pos.x() as i32),
                            y: Some(pawn.pos.y() as i32),
                            black: Some(pawn.color == PawnColor::Black),
                        })
                        .collect(),
                    black_turn: Some(onoro.player_color() == PawnColor::Black),
                    turn_num: Some(onoro.pawns_in_play() - 1),
                    finished: Some(onoro.finished().is_some()),
                },
            }
        }
        pub fn to_onoro<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
            &self,
        ) -> Result<Onoro<N, N2, ADJ_CNT_SIZE>, Error> {
            let mut black_moves = Vec::new();
            let mut while_moves = Vec::new();
            let [min_x, min_y] = self
                .game_state
                .pawns
                .iter()
                .filter_map(|pawn| Some([pawn.x?, pawn.y?]))
                .reduce(|[min_x, min_y], [x, y]| [min_x.min(x), min_y.min(y)])
                .map(Ok)
                .unwrap_or(Err(Error::ProtoDecode("No valid pawns".into())))?;
            for pawn_proto in &self.game_state.pawns {
                let x = (pawn_proto.x().wrapping_sub(min_x).wrapping_add(1)) as u32;
                let y = (pawn_proto.y().wrapping_sub(min_y).wrapping_add(1)) as u32;
                if x >= N as u32 || y >= N as u32 {
                    return Err(
                        Error::ProtoDecode({
                            let res = ::alloc::fmt::format(
                                format_args!(
                                    "x/y out of bounds: {0} {1}",
                                    pawn_proto.x(),
                                    pawn_proto.y(),
                                ),
                            );
                            res
                        }),
                    );
                }
                let m = Move::Phase1Move {
                    to: PackedIdx::new(x, y),
                };
                if pawn_proto.black() {
                    black_moves.push(m);
                } else {
                    while_moves.push(m);
                }
            }
            if black_moves.len() > N || while_moves.len() > N {
                return Err(
                    Error::ProtoDecode({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "Too many pawns in board: {0} black and {1} white",
                                black_moves.len(),
                                while_moves.len(),
                            ),
                        );
                        res
                    }),
                );
            }
            if black_moves.is_empty() {
                return Err(
                    Error::ProtoDecode(
                        "Must have at least one black pawn placed, since they are the first player."
                            .into(),
                    ),
                );
            }
            if !((black_moves.len() - 1)..=black_moves.len())
                .contains(&while_moves.len())
            {
                return Err(
                    Error::ProtoDecode({
                        let res = ::alloc::fmt::format(
                            format_args!(
                                "There must be either one fewer or equally many white pawns as there are black. Found {0} black and {1} white.",
                                black_moves.len(),
                                while_moves.len(),
                            ),
                        );
                        res
                    }),
                );
            }
            let mut game = unsafe { Onoro::new() };
            unsafe {
                game.make_move_unchecked(black_moves[0]);
            }
            for m in interleave(while_moves, black_moves.into_iter().skip(1)) {
                game.make_move(m);
            }
            Ok(game)
        }
    }
    impl<
        const N: usize,
        const N2: usize,
        const ADJ_CNT_SIZE: usize,
    > From<&Onoro<N, N2, ADJ_CNT_SIZE>> for GameStateProto {
        fn from(onoro: &Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
            Self::from_onoro(onoro)
        }
    }
    impl Serialize for GameStateProto {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut buf = BytesMut::new();
            self.game_state.encode(&mut buf).map_err(ser::Error::custom)?;
            serializer.serialize_bytes(&buf)
        }
    }
    impl<'de> Deserialize<'de> for GameStateProto {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let buf = deserializer.deserialize_bytes(BytesMutVisitor)?;
            let game_state = proto_impl::GameState::decode(buf)
                .map_err(de::Error::custom)?;
            Ok(GameStateProto { game_state })
        }
    }
}
mod socket_init {
    use std::time::Duration;
    use async_sockets::{
        AsyncSocket, AsyncSocketContext, AsyncSocketEmitters, AsyncSocketListeners,
        AsyncSocketOptions, AsyncSocketResponders, Status,
    };
    use onoro::Onoro16;
    use tokio::task::JoinHandle;
    use crate::proto::GameStateProto;
    enum ServerEmitEvents {}
    impl ::serde::ser::Serialize for ServerEmitEvents {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            match *self {}
        }
    }
    impl ::async_sockets::SerMessage for ServerEmitEvents {}
    enum ClientEmitEvents {}
    impl<'de> ::serde::de::Deserialize<'de> for ClientEmitEvents {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            #[serde(
                rename_all = "snake_case",
                tag = "event",
                content = "args",
                deny_unknown_fields
            )]
            enum AsyncSocketInternalListenerEvent {}
            #[doc(hidden)]
            #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
            const _: () = {
                #[allow(unused_extern_crates, clippy::useless_attribute)]
                extern crate serde as _serde;
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for AsyncSocketInternalListenerEvent {
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        #[allow(non_camel_case_types)]
                        #[doc(hidden)]
                        enum __Field {}
                        #[doc(hidden)]
                        struct __FieldVisitor;
                        impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                            type Value = __Field;
                            fn expecting(
                                &self,
                                __formatter: &mut _serde::__private::Formatter,
                            ) -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(
                                    __formatter,
                                    "variant identifier",
                                )
                            }
                            fn visit_u64<__E>(
                                self,
                                __value: u64,
                            ) -> _serde::__private::Result<Self::Value, __E>
                            where
                                __E: _serde::de::Error,
                            {
                                match __value {
                                    _ => {
                                        _serde::__private::Err(
                                            _serde::de::Error::invalid_value(
                                                _serde::de::Unexpected::Unsigned(__value),
                                                &"variant index 0 <= i < 0",
                                            ),
                                        )
                                    }
                                }
                            }
                            fn visit_str<__E>(
                                self,
                                __value: &str,
                            ) -> _serde::__private::Result<Self::Value, __E>
                            where
                                __E: _serde::de::Error,
                            {
                                match __value {
                                    _ => {
                                        _serde::__private::Err(
                                            _serde::de::Error::unknown_variant(__value, VARIANTS),
                                        )
                                    }
                                }
                            }
                            fn visit_bytes<__E>(
                                self,
                                __value: &[u8],
                            ) -> _serde::__private::Result<Self::Value, __E>
                            where
                                __E: _serde::de::Error,
                            {
                                match __value {
                                    _ => {
                                        let __value = &_serde::__private::from_utf8_lossy(__value);
                                        _serde::__private::Err(
                                            _serde::de::Error::unknown_variant(__value, VARIANTS),
                                        )
                                    }
                                }
                            }
                        }
                        impl<'de> _serde::Deserialize<'de> for __Field {
                            #[inline]
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::Deserializer::deserialize_identifier(
                                    __deserializer,
                                    __FieldVisitor,
                                )
                            }
                        }
                        #[doc(hidden)]
                        const VARIANTS: &'static [&'static str] = &[];
                        #[doc(hidden)]
                        struct __Seed<'de> {
                            field: __Field,
                            marker: _serde::__private::PhantomData<
                                AsyncSocketInternalListenerEvent,
                            >,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl<'de> _serde::de::DeserializeSeed<'de> for __Seed<'de> {
                            type Value = AsyncSocketInternalListenerEvent;
                            fn deserialize<__D>(
                                self,
                                __deserializer: __D,
                            ) -> _serde::__private::Result<Self::Value, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                match self.field {}
                            }
                        }
                        #[doc(hidden)]
                        struct __Visitor<'de> {
                            marker: _serde::__private::PhantomData<
                                AsyncSocketInternalListenerEvent,
                            >,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                            type Value = AsyncSocketInternalListenerEvent;
                            fn expecting(
                                &self,
                                __formatter: &mut _serde::__private::Formatter,
                            ) -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(
                                    __formatter,
                                    "adjacently tagged enum AsyncSocketInternalListenerEvent",
                                )
                            }
                            fn visit_map<__A>(
                                self,
                                mut __map: __A,
                            ) -> _serde::__private::Result<Self::Value, __A::Error>
                            where
                                __A: _serde::de::MapAccess<'de>,
                            {
                                match _serde::de::MapAccess::next_key_seed(
                                    &mut __map,
                                    _serde::__private::de::TagOrContentFieldVisitor {
                                        tag: "event",
                                        content: "args",
                                    },
                                )? {
                                    _serde::__private::Some(
                                        _serde::__private::de::TagOrContentField::Tag,
                                    ) => {
                                        let __field = _serde::de::MapAccess::next_value_seed(
                                            &mut __map,
                                            _serde::__private::de::AdjacentlyTaggedEnumVariantSeed::<
                                                __Field,
                                            > {
                                                enum_name: "AsyncSocketInternalListenerEvent",
                                                variants: VARIANTS,
                                                fields_enum: _serde::__private::PhantomData,
                                            },
                                        )?;
                                        match _serde::de::MapAccess::next_key_seed(
                                            &mut __map,
                                            _serde::__private::de::TagOrContentFieldVisitor {
                                                tag: "event",
                                                content: "args",
                                            },
                                        )? {
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Tag,
                                            ) => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::duplicate_field("event"),
                                                )
                                            }
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Content,
                                            ) => {
                                                let __ret = _serde::de::MapAccess::next_value_seed(
                                                    &mut __map,
                                                    __Seed {
                                                        field: __field,
                                                        marker: _serde::__private::PhantomData,
                                                        lifetime: _serde::__private::PhantomData,
                                                    },
                                                )?;
                                                match _serde::de::MapAccess::next_key_seed(
                                                    &mut __map,
                                                    _serde::__private::de::TagOrContentFieldVisitor {
                                                        tag: "event",
                                                        content: "args",
                                                    },
                                                )? {
                                                    _serde::__private::Some(
                                                        _serde::__private::de::TagOrContentField::Tag,
                                                    ) => {
                                                        _serde::__private::Err(
                                                            <__A::Error as _serde::de::Error>::duplicate_field("event"),
                                                        )
                                                    }
                                                    _serde::__private::Some(
                                                        _serde::__private::de::TagOrContentField::Content,
                                                    ) => {
                                                        _serde::__private::Err(
                                                            <__A::Error as _serde::de::Error>::duplicate_field("args"),
                                                        )
                                                    }
                                                    _serde::__private::None => _serde::__private::Ok(__ret),
                                                }
                                            }
                                            _serde::__private::None => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::missing_field("args"),
                                                )
                                            }
                                        }
                                    }
                                    _serde::__private::Some(
                                        _serde::__private::de::TagOrContentField::Content,
                                    ) => {
                                        let __content = _serde::de::MapAccess::next_value::<
                                            _serde::__private::de::Content,
                                        >(&mut __map)?;
                                        match _serde::de::MapAccess::next_key_seed(
                                            &mut __map,
                                            _serde::__private::de::TagOrContentFieldVisitor {
                                                tag: "event",
                                                content: "args",
                                            },
                                        )? {
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Tag,
                                            ) => {
                                                let __deserializer = _serde::__private::de::ContentDeserializer::<
                                                    __A::Error,
                                                >::new(__content);
                                                match _serde::de::MapAccess::next_value_seed(
                                                    &mut __map,
                                                    _serde::__private::de::AdjacentlyTaggedEnumVariantSeed::<
                                                        __Field,
                                                    > {
                                                        enum_name: "AsyncSocketInternalListenerEvent",
                                                        variants: VARIANTS,
                                                        fields_enum: _serde::__private::PhantomData,
                                                    },
                                                )? {}
                                            }
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Content,
                                            ) => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::duplicate_field("args"),
                                                )
                                            }
                                            _serde::__private::None => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::missing_field("event"),
                                                )
                                            }
                                        }
                                    }
                                    _serde::__private::None => {
                                        _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::missing_field("event"),
                                        )
                                    }
                                }
                            }
                            fn visit_seq<__A>(
                                self,
                                mut __seq: __A,
                            ) -> _serde::__private::Result<Self::Value, __A::Error>
                            where
                                __A: _serde::de::SeqAccess<'de>,
                            {
                                match _serde::de::SeqAccess::next_element(&mut __seq)? {
                                    _serde::__private::Some(__field) => {
                                        match _serde::de::SeqAccess::next_element_seed(
                                            &mut __seq,
                                            __Seed {
                                                field: __field,
                                                marker: _serde::__private::PhantomData,
                                                lifetime: _serde::__private::PhantomData,
                                            },
                                        )? {
                                            _serde::__private::Some(__ret) => {
                                                _serde::__private::Ok(__ret)
                                            }
                                            _serde::__private::None => {
                                                _serde::__private::Err(
                                                    _serde::de::Error::invalid_length(1, &self),
                                                )
                                            }
                                        }
                                    }
                                    _serde::__private::None => {
                                        _serde::__private::Err(
                                            _serde::de::Error::invalid_length(0, &self),
                                        )
                                    }
                                }
                            }
                        }
                        #[doc(hidden)]
                        const FIELDS: &'static [&'static str] = &["event", "args"];
                        _serde::Deserializer::deserialize_struct(
                            __deserializer,
                            "AsyncSocketInternalListenerEvent",
                            FIELDS,
                            __Visitor {
                                marker: _serde::__private::PhantomData::<
                                    AsyncSocketInternalListenerEvent,
                                >,
                                lifetime: _serde::__private::PhantomData,
                            },
                        )
                    }
                }
            };
            let proxy = AsyncSocketInternalListenerEvent::deserialize(deserializer)?;
            match proxy {}
        }
    }
    impl ::async_sockets::DeMessage for ClientEmitEvents {}
    enum ToClientRequests {}
    impl ::serde::ser::Serialize for ToClientRequests {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            match *self {}
        }
    }
    impl ::async_sockets::SerMessage for ToClientRequests {}
    enum FromClientResponses {}
    enum FromClientRequests {
        NewGame {},
    }
    impl<'de> ::serde::de::Deserialize<'de> for FromClientRequests {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            #[serde(
                rename_all = "snake_case",
                tag = "event",
                content = "args",
                deny_unknown_fields
            )]
            enum AsyncSocketInternalListenerEvent {
                NewGame(()),
            }
            #[doc(hidden)]
            #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
            const _: () = {
                #[allow(unused_extern_crates, clippy::useless_attribute)]
                extern crate serde as _serde;
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for AsyncSocketInternalListenerEvent {
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        #[allow(non_camel_case_types)]
                        #[doc(hidden)]
                        enum __Field {
                            __field0,
                        }
                        #[doc(hidden)]
                        struct __FieldVisitor;
                        impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                            type Value = __Field;
                            fn expecting(
                                &self,
                                __formatter: &mut _serde::__private::Formatter,
                            ) -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(
                                    __formatter,
                                    "variant identifier",
                                )
                            }
                            fn visit_u64<__E>(
                                self,
                                __value: u64,
                            ) -> _serde::__private::Result<Self::Value, __E>
                            where
                                __E: _serde::de::Error,
                            {
                                match __value {
                                    0u64 => _serde::__private::Ok(__Field::__field0),
                                    _ => {
                                        _serde::__private::Err(
                                            _serde::de::Error::invalid_value(
                                                _serde::de::Unexpected::Unsigned(__value),
                                                &"variant index 0 <= i < 1",
                                            ),
                                        )
                                    }
                                }
                            }
                            fn visit_str<__E>(
                                self,
                                __value: &str,
                            ) -> _serde::__private::Result<Self::Value, __E>
                            where
                                __E: _serde::de::Error,
                            {
                                match __value {
                                    "new_game" => _serde::__private::Ok(__Field::__field0),
                                    _ => {
                                        _serde::__private::Err(
                                            _serde::de::Error::unknown_variant(__value, VARIANTS),
                                        )
                                    }
                                }
                            }
                            fn visit_bytes<__E>(
                                self,
                                __value: &[u8],
                            ) -> _serde::__private::Result<Self::Value, __E>
                            where
                                __E: _serde::de::Error,
                            {
                                match __value {
                                    b"new_game" => _serde::__private::Ok(__Field::__field0),
                                    _ => {
                                        let __value = &_serde::__private::from_utf8_lossy(__value);
                                        _serde::__private::Err(
                                            _serde::de::Error::unknown_variant(__value, VARIANTS),
                                        )
                                    }
                                }
                            }
                        }
                        impl<'de> _serde::Deserialize<'de> for __Field {
                            #[inline]
                            fn deserialize<__D>(
                                __deserializer: __D,
                            ) -> _serde::__private::Result<Self, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                _serde::Deserializer::deserialize_identifier(
                                    __deserializer,
                                    __FieldVisitor,
                                )
                            }
                        }
                        #[doc(hidden)]
                        const VARIANTS: &'static [&'static str] = &["new_game"];
                        #[doc(hidden)]
                        struct __Seed<'de> {
                            field: __Field,
                            marker: _serde::__private::PhantomData<
                                AsyncSocketInternalListenerEvent,
                            >,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl<'de> _serde::de::DeserializeSeed<'de> for __Seed<'de> {
                            type Value = AsyncSocketInternalListenerEvent;
                            fn deserialize<__D>(
                                self,
                                __deserializer: __D,
                            ) -> _serde::__private::Result<Self::Value, __D::Error>
                            where
                                __D: _serde::Deserializer<'de>,
                            {
                                match self.field {
                                    __Field::__field0 => {
                                        _serde::__private::Result::map(
                                            <() as _serde::Deserialize>::deserialize(__deserializer),
                                            AsyncSocketInternalListenerEvent::NewGame,
                                        )
                                    }
                                }
                            }
                        }
                        #[doc(hidden)]
                        struct __Visitor<'de> {
                            marker: _serde::__private::PhantomData<
                                AsyncSocketInternalListenerEvent,
                            >,
                            lifetime: _serde::__private::PhantomData<&'de ()>,
                        }
                        impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                            type Value = AsyncSocketInternalListenerEvent;
                            fn expecting(
                                &self,
                                __formatter: &mut _serde::__private::Formatter,
                            ) -> _serde::__private::fmt::Result {
                                _serde::__private::Formatter::write_str(
                                    __formatter,
                                    "adjacently tagged enum AsyncSocketInternalListenerEvent",
                                )
                            }
                            fn visit_map<__A>(
                                self,
                                mut __map: __A,
                            ) -> _serde::__private::Result<Self::Value, __A::Error>
                            where
                                __A: _serde::de::MapAccess<'de>,
                            {
                                match _serde::de::MapAccess::next_key_seed(
                                    &mut __map,
                                    _serde::__private::de::TagOrContentFieldVisitor {
                                        tag: "event",
                                        content: "args",
                                    },
                                )? {
                                    _serde::__private::Some(
                                        _serde::__private::de::TagOrContentField::Tag,
                                    ) => {
                                        let __field = _serde::de::MapAccess::next_value_seed(
                                            &mut __map,
                                            _serde::__private::de::AdjacentlyTaggedEnumVariantSeed::<
                                                __Field,
                                            > {
                                                enum_name: "AsyncSocketInternalListenerEvent",
                                                variants: VARIANTS,
                                                fields_enum: _serde::__private::PhantomData,
                                            },
                                        )?;
                                        match _serde::de::MapAccess::next_key_seed(
                                            &mut __map,
                                            _serde::__private::de::TagOrContentFieldVisitor {
                                                tag: "event",
                                                content: "args",
                                            },
                                        )? {
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Tag,
                                            ) => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::duplicate_field("event"),
                                                )
                                            }
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Content,
                                            ) => {
                                                let __ret = _serde::de::MapAccess::next_value_seed(
                                                    &mut __map,
                                                    __Seed {
                                                        field: __field,
                                                        marker: _serde::__private::PhantomData,
                                                        lifetime: _serde::__private::PhantomData,
                                                    },
                                                )?;
                                                match _serde::de::MapAccess::next_key_seed(
                                                    &mut __map,
                                                    _serde::__private::de::TagOrContentFieldVisitor {
                                                        tag: "event",
                                                        content: "args",
                                                    },
                                                )? {
                                                    _serde::__private::Some(
                                                        _serde::__private::de::TagOrContentField::Tag,
                                                    ) => {
                                                        _serde::__private::Err(
                                                            <__A::Error as _serde::de::Error>::duplicate_field("event"),
                                                        )
                                                    }
                                                    _serde::__private::Some(
                                                        _serde::__private::de::TagOrContentField::Content,
                                                    ) => {
                                                        _serde::__private::Err(
                                                            <__A::Error as _serde::de::Error>::duplicate_field("args"),
                                                        )
                                                    }
                                                    _serde::__private::None => _serde::__private::Ok(__ret),
                                                }
                                            }
                                            _serde::__private::None => {
                                                match __field {
                                                    __Field::__field0 => {
                                                        _serde::__private::de::missing_field("args")
                                                            .map(AsyncSocketInternalListenerEvent::NewGame)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _serde::__private::Some(
                                        _serde::__private::de::TagOrContentField::Content,
                                    ) => {
                                        let __content = _serde::de::MapAccess::next_value::<
                                            _serde::__private::de::Content,
                                        >(&mut __map)?;
                                        match _serde::de::MapAccess::next_key_seed(
                                            &mut __map,
                                            _serde::__private::de::TagOrContentFieldVisitor {
                                                tag: "event",
                                                content: "args",
                                            },
                                        )? {
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Tag,
                                            ) => {
                                                let __deserializer = _serde::__private::de::ContentDeserializer::<
                                                    __A::Error,
                                                >::new(__content);
                                                let __ret = match _serde::de::MapAccess::next_value_seed(
                                                    &mut __map,
                                                    _serde::__private::de::AdjacentlyTaggedEnumVariantSeed::<
                                                        __Field,
                                                    > {
                                                        enum_name: "AsyncSocketInternalListenerEvent",
                                                        variants: VARIANTS,
                                                        fields_enum: _serde::__private::PhantomData,
                                                    },
                                                )? {
                                                    __Field::__field0 => {
                                                        _serde::__private::Result::map(
                                                            <() as _serde::Deserialize>::deserialize(__deserializer),
                                                            AsyncSocketInternalListenerEvent::NewGame,
                                                        )
                                                    }
                                                }?;
                                                match _serde::de::MapAccess::next_key_seed(
                                                    &mut __map,
                                                    _serde::__private::de::TagOrContentFieldVisitor {
                                                        tag: "event",
                                                        content: "args",
                                                    },
                                                )? {
                                                    _serde::__private::Some(
                                                        _serde::__private::de::TagOrContentField::Tag,
                                                    ) => {
                                                        _serde::__private::Err(
                                                            <__A::Error as _serde::de::Error>::duplicate_field("event"),
                                                        )
                                                    }
                                                    _serde::__private::Some(
                                                        _serde::__private::de::TagOrContentField::Content,
                                                    ) => {
                                                        _serde::__private::Err(
                                                            <__A::Error as _serde::de::Error>::duplicate_field("args"),
                                                        )
                                                    }
                                                    _serde::__private::None => _serde::__private::Ok(__ret),
                                                }
                                            }
                                            _serde::__private::Some(
                                                _serde::__private::de::TagOrContentField::Content,
                                            ) => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::duplicate_field("args"),
                                                )
                                            }
                                            _serde::__private::None => {
                                                _serde::__private::Err(
                                                    <__A::Error as _serde::de::Error>::missing_field("event"),
                                                )
                                            }
                                        }
                                    }
                                    _serde::__private::None => {
                                        _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::missing_field("event"),
                                        )
                                    }
                                }
                            }
                            fn visit_seq<__A>(
                                self,
                                mut __seq: __A,
                            ) -> _serde::__private::Result<Self::Value, __A::Error>
                            where
                                __A: _serde::de::SeqAccess<'de>,
                            {
                                match _serde::de::SeqAccess::next_element(&mut __seq)? {
                                    _serde::__private::Some(__field) => {
                                        match _serde::de::SeqAccess::next_element_seed(
                                            &mut __seq,
                                            __Seed {
                                                field: __field,
                                                marker: _serde::__private::PhantomData,
                                                lifetime: _serde::__private::PhantomData,
                                            },
                                        )? {
                                            _serde::__private::Some(__ret) => {
                                                _serde::__private::Ok(__ret)
                                            }
                                            _serde::__private::None => {
                                                _serde::__private::Err(
                                                    _serde::de::Error::invalid_length(1, &self),
                                                )
                                            }
                                        }
                                    }
                                    _serde::__private::None => {
                                        _serde::__private::Err(
                                            _serde::de::Error::invalid_length(0, &self),
                                        )
                                    }
                                }
                            }
                        }
                        #[doc(hidden)]
                        const FIELDS: &'static [&'static str] = &["event", "args"];
                        _serde::Deserializer::deserialize_struct(
                            __deserializer,
                            "AsyncSocketInternalListenerEvent",
                            FIELDS,
                            __Visitor {
                                marker: _serde::__private::PhantomData::<
                                    AsyncSocketInternalListenerEvent,
                                >,
                                lifetime: _serde::__private::PhantomData,
                            },
                        )
                    }
                }
            };
            let proxy = AsyncSocketInternalListenerEvent::deserialize(deserializer)?;
            match proxy {
                AsyncSocketInternalListenerEvent::NewGame(()) => {
                    Ok(FromClientRequests::NewGame {})
                }
            }
        }
    }
    impl ::async_sockets::DeMessage for FromClientRequests {}
    enum ToClientResponses {
        NewGame { game: GameStateProto },
    }
    impl ::serde::ser::Serialize for ToClientResponses {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            match self {
                ToClientResponses::NewGame { game } => {
                    #[serde(deny_unknown_fields)]
                    struct AsyncSocketInternalRespondEvent<'a> {
                        game: &'a GameStateProto,
                    }
                    #[doc(hidden)]
                    #[allow(
                        non_upper_case_globals,
                        unused_attributes,
                        unused_qualifications
                    )]
                    const _: () = {
                        #[allow(unused_extern_crates, clippy::useless_attribute)]
                        extern crate serde as _serde;
                        #[automatically_derived]
                        impl<'a> _serde::Serialize
                        for AsyncSocketInternalRespondEvent<'a> {
                            fn serialize<__S>(
                                &self,
                                __serializer: __S,
                            ) -> _serde::__private::Result<__S::Ok, __S::Error>
                            where
                                __S: _serde::Serializer,
                            {
                                let mut __serde_state = _serde::Serializer::serialize_struct(
                                    __serializer,
                                    "AsyncSocketInternalRespondEvent",
                                    false as usize + 1,
                                )?;
                                _serde::ser::SerializeStruct::serialize_field(
                                    &mut __serde_state,
                                    "game",
                                    &self.game,
                                )?;
                                _serde::ser::SerializeStruct::end(__serde_state)
                            }
                        }
                    };
                    AsyncSocketInternalRespondEvent {
                        game,
                    }
                        .serialize(serializer)
                }
            }
        }
    }
    impl ::async_sockets::SerMessage for ToClientResponses {}
    async fn handle_connect_event(_context: AsyncSocketContext<ServerEmitEvents>) {}
    async fn handle_call_event(
        event: FromClientRequests,
        _context: AsyncSocketContext<ServerEmitEvents>,
    ) -> Status<ToClientResponses> {
        match event {
            FromClientRequests::NewGame {} => {
                Status::Ok(ToClientResponses::NewGame {
                    game: GameStateProto::from_onoro(&Onoro16::default_start()),
                })
            }
        }
    }
    async fn handle_emit_event(
        event: ClientEmitEvents,
        _context: AsyncSocketContext<ServerEmitEvents>,
    ) {
        match event {}
    }
    pub fn create_socket_endpoint() -> JoinHandle<()> {
        tokio::spawn(async {
            AsyncSocket::new(
                    AsyncSocketOptions::new()
                        .with_path("onoro")
                        .with_port(2345)
                        .with_timeout(Duration::from_secs(10)),
                    handle_connect_event,
                    handle_emit_event,
                    handle_call_event,
                )
                .start_server()
                .await
        })
    }
}
fn main() {
    let body = async {
        initialize::init().await;
    };
    #[allow(clippy::expect_used, clippy::diverging_sub_expression)]
    {
        return tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body);
    }
}
