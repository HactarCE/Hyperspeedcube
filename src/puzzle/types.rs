use enum_map::Enum;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{traits::*, Face, Piece, Rubiks24, Rubiks33, Rubiks34, Sticker};

/// Declare the `PuzzleType` enum and also generate a `with_puzzle_types` macro.
macro_rules! puzzle_type_enum {
    (
        @ [$dollar:tt]
        $( #[$_outer_attr:meta] )*
        $_vis:vis enum PuzzleType {
            $( $( #[$_inner_attr:meta] )* $puzzle_type:ident ),* $(,)?
        }
    ) => {
        /// Replaces `PUZZLE_TYPES` argument with a comma-separated list of
        /// puzzle types.
        macro_rules! with_puzzle_types {
            (
                $dollar callback:ident ! {
                    $dollar( @ $dollar instruction:tt )?
                    puzzle_types = PUZZLE_TYPES
                    $dollar( $dollar arg:tt )*
                }
            ) => {
                $dollar callback! {
                    $dollar( @ $dollar instruction )?
                    puzzle_types = {[ $($puzzle_type),* ]}
                    $dollar( $dollar arg )*
                }
            }
        }
    };
    ( $($tok:tt)* ) => {
        // Make the enum definition.
        $($tok)*
        // Make a macro for getting a list of all puzzles.
        puzzle_type_enum! { @ [$] $($tok)* }
    };
}
puzzle_type_enum! {
    /// Enumeration of all puzzle types.
    #[derive(Enum, EnumIter, EnumString, Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum PuzzleType {
        /// 3x3x3 (3^3) Rubik's cube.
        Rubiks33,
        /// 3x3x3x3 (3^4) Rubik's cube.
        Rubiks34,
        /// 2x2x2x2 (2^4) Rubik's cube.
        Rubiks24,
    }
}
impl fmt::Display for PuzzleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
impl AsRef<str> for PuzzleType {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

macro_rules! delegate_to_puzzle_type {
    (
        @expr
        puzzle_type = {[ $puzzle_type:ident ]}
        type_name = {[ $type_name:ident ]}
        foreach = {[ $($foreach:tt)* ]}
    ) => {
        {
            type $type_name = $puzzle_type;
            $($foreach)*
        }
    };
    (
        puzzle_types = {[ $($puzzle_type:ident),* ]}
        match_expr = {[ $match_expr:expr ]}
        type_name = $type_name:tt
        foreach = $foreach:tt
    ) => {
        match $match_expr {
            $(
                PuzzleType::$puzzle_type => delegate_to_puzzle_type! {
                    @expr
                    puzzle_type = {[ $puzzle_type ]}
                    type_name = $type_name
                    foreach = $foreach
                },
            )*
        }
    };

    (
        match_expr = $match_expr:tt
        type_name = {[ $type_name:ident ]}
        method_name = {[ $method_name:tt ]}
        args = {[ (&self $(, $arg_name:ident : $_arg_ty:ty )* $(,)?) ]}
    ) => {
        delegate_to_puzzle_type! {
            match_expr = $match_expr
            type_name = {[ $type_name ]}
            // Automatically convert arguments and return type if necessary.
            foreach = {[ $type_name::$method_name($( $arg_name.try_into().unwrap() ),*).try_into().unwrap() ]}
        }
    };
    ( match_expr $( $arg:tt )* ) => {
        with_puzzle_types! {
            delegate_to_puzzle_type! {
                puzzle_types = PUZZLE_TYPES
                match_expr $( $arg )*
            }
        }
    };
}

macro_rules! delegate_fn_to_puzzle_type {
    (
        type $type_name:ident = match $match_expr:expr;
        $( #[$attr:meta] )* $v:vis fn $method_name:ident $args:tt $( -> $ret:ty )?
        { $( $foreach:tt )* }
        $(;)?
    ) => {
        $( #[$attr] )* $v fn $method_name $args $(-> $ret)? {
            delegate_to_puzzle_type! {
                match_expr = {[ $match_expr ]}
                type_name = {[ $type_name ]}
                foreach = {[ $( $foreach )* ]}
            }
        }
    };
    (
        type $type_name:ident = match $match_expr:expr;
        $( #[$attr:meta] )* $v:vis fn $method_name:ident $args:tt $( -> $ret:ty )?
        $(;)?
    ) => {
        $( #[$attr] )* $v fn $method_name $args $( -> $ret )? {
            delegate_to_puzzle_type! {
                match_expr = {[ $match_expr ]}
                type_name = {[ $type_name ]}
                method_name = {[ $method_name ]}
                args = {[ $args ]}
            }
        }
    };

    // Convert one massive macro call into a single macro call per method
    (
        type $type_name:ident = match $match_expr:expr;
        $(
            $( #[$attr:meta] )* $v:vis fn $method_name:ident $args:tt $( -> $ret:ty )?
            $( { $( $body_tok:tt )* } )?
            $(;)?
        )*
    ) => {
        $(
            delegate_fn_to_puzzle_type! {
                type $type_name = match $match_expr;
                $( #[$attr] )* $v fn $method_name $args $( -> $ret )?
                $( { $( $body_tok )* } )?
            }
        )*
    };
}

macro_rules! puzzle_type_list {
    ( puzzle_types = {[ $( $puzzle_type:ident ),* ]} ) => {
        [ $( PuzzleType::$puzzle_type ),* ]
    };
    () => {
        with_puzzle_types! {
            puzzle_type_list! { puzzle_types = PUZZLE_TYPES }
        }
    };
}

impl PuzzleType {
    /// List of all puzzle types.
    pub const ALL: &'static [Self] = &puzzle_type_list!();
}
impl PuzzleTypeTrait for PuzzleType {
    delegate_fn_to_puzzle_type! {
        type P = match self;

        fn name(&self) -> &'static str {
            P::NAME
        }
        fn ndim(&self) -> usize {
            P::NDIM
        }
        fn layer_count(&self) -> usize {
            P::LAYER_COUNT
        }
        fn scramble_moves_count(&self) -> usize {
            P::SCRAMBLE_MOVES_COUNT
        }

        fn pieces(&self) -> &'static [Piece] {
            P::generic_pieces()
        }
        fn stickers(&self) -> &'static [Sticker] {
            P::generic_stickers()
        }
        fn faces(&self) -> &'static [Face] {
            P::generic_faces()
        }

        fn face_symbols(&self) -> &'static [&'static str];
        fn face_names(&self) -> &'static [&'static str];
        fn piece_type_names(&self) -> &'static [&'static str] {
            P::PIECE_TYPE_NAMES
        }

        fn twist_direction_symbols(&self) -> &'static [&'static str];
        fn twist_direction_names(&self) -> &'static [&'static str];
    }
}

impl Default for PuzzleType {
    fn default() -> Self {
        PuzzleType::Rubiks24
    }
}
