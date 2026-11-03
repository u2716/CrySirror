// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS natives acknowledged without their effect because the capability behind them
//! is not connected: movies, particles, 3D models, text fonts, data tables and
//! other task classes. The result slot of each comes from its reviewed native body;
//! a value answers 0, which scripts treat as absent or finished. The first call of
//! each id is logged, so a running game shows what it is missing.
/// `Some(has_value)` for a listed native.
pub(crate) fn slot(id: u32) -> Option<bool> {
    match id {
        200 | 241 | 243 | 253 | 271 | 274 | 364 | 385 | 386 | 514 | 576 | 601 | 608 | 635 | 636 | 639 | 640 | 649 | 683 | 685 | 688 | 689 | 691 | 693 | 702 | 869 | 872 | 873 | 979 | 980 | 989 | 992 | 998 | 1009 | 1022 | 1027 | 1028 | 1029 | 1033 | 1049 => Some(true),
        81 | 196 | 197 | 201 | 203 | 272 | 273 | 275 | 279 | 280 | 281 | 282 | 283 | 287 | 288 | 317 | 366 | 369 | 398 | 399 | 427 | 433 | 521 | 522 | 523 | 524 | 538 | 541 | 570 | 600 | 611 | 612 | 613 | 614 | 615 | 617 | 619 | 620 | 621 | 634 | 681 | 684 | 690 | 692 | 699 | 701 | 735 | 740 | 853 | 854 | 855 | 856 | 857 | 858 | 859 | 860 | 861 | 870 | 878 | 879 | 881 | 905 | 910 | 924 | 929 | 930 | 965 | 967 | 968 | 970 | 971 | 972 | 978 | 981 | 988 | 993 | 994 | 1018 | 1019 | 1020 | 1021 | 1023 | 1025 | 1030 | 1034 | 1047 | 1048 | 76 | 567 | 577 | 628 | 846 | 871 => Some(false),
        _ => None,
    }
}
