hl.config({
    general = {
        gaps_in          = 5,
        gaps_out         = 25,
        border_size      = 0,
        resize_on_border = false,
        allow_tearing    = false,
        layout           = "dwindle",
    },

    decoration = {
        rounding         = 12,
        rounding_power   = 2,
        active_opacity   = 1.0,
        inactive_opacity = 0.9,
        shadow           = {
            enabled      = true
        },
        blur             = {
            enabled  = true,
            size     = 2,
            passes   = 2,
            vibrancy = 0.17,
        },
    },
})
