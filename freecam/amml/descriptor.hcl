namespace         = "alaydriem"
name              = "freecam"
display_name      = "Freecam"
version           = "0.1.0"
author            = "Alaydriem"
config_path       = "config.hcl"
input_passthrough = [27, 112]

claim "Camera" {
  mode   = "Shared"
  reason = "Flies the camera while freecam is on"
}
claim "Input" {
  mode              = "Shared"
  reason            = "Flight keys and mouse look"
  block_player_turn = true
}
claim "Hud" {
  mode             = "Shared"
  reason           = "Hides the vanilla HUD while flying"
  hide_vanilla_hud = true
}
claim "Overlay" {
  mode   = "Shared"
  reason = "Strip, legend and playback bar"
}
claim "WorldDraw" {
  mode   = "Shared"
  reason = "Path markers"
}

keybind "toggle" {
  label       = "Toggle freecam"
  default     = "F3+P"
  capability  = "Camera"
  scope       = "Global"
  config_path = "keys.toggle"
}

keybind "toggle_smooth" {
  label       = "Toggle smooth camera"
  default     = "Ctrl+LeftBracket"
  capability  = "Camera"
  scope       = "WhileHeld"
  config_path = "keys.toggle_smooth"
}

keybind "add_point" {
  label       = "Add path point"
  default     = "Ctrl+B"
  capability  = "Camera"
  scope       = "WhileHeld"
  config_path = "keys.add_point"
}

keybind "toggle_bar" {
  label       = "Show or hide the playback bar"
  default     = "F3+E"
  capability  = "Camera"
  scope       = "WhileHeld"
  config_path = "keys.toggle_bar"
}

keybind "delete_point" {
  label       = "Delete path point"
  default     = "F3+Delete"
  capability  = "Camera"
  scope       = "WhileHeld"
  config_path = "keys.delete_point"
}

keybind "fly_forward" {
  label       = "Fly forward"
  default     = "W"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_forward"
}

keybind "fly_back" {
  label       = "Fly back"
  default     = "S"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_back"
}

keybind "fly_left" {
  label       = "Fly left"
  default     = "A"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_left"
}

keybind "fly_right" {
  label       = "Fly right"
  default     = "D"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_right"
}

keybind "fly_up" {
  label       = "Fly up"
  default     = "Space"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_up"
}

keybind "fly_down" {
  label       = "Fly down"
  default     = "Shift"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_down"
}

keybind "fly_fast" {
  label       = "Fly fast"
  default     = "Ctrl"
  capability  = "Input"
  scope       = "WhileHeld"
  config_path = "keys.fly_fast"
}

section "keys" {
  title = "Keys"

  field "keys.toggle" {
    label   = "Toggle freecam"
    kind    = "chord"
    default = "F3+P"
    apply   = "Live"
  }

  field "keys.toggle_smooth" {
    label   = "Toggle smooth camera"
    kind    = "chord"
    default = "Ctrl+LeftBracket"
    apply   = "Live"
  }

  field "keys.add_point" {
    label   = "Add path point"
    kind    = "chord"
    default = "Ctrl+B"
    apply   = "Live"
  }

  field "keys.toggle_bar" {
    label   = "Show or hide the playback bar"
    kind    = "chord"
    default = "F3+E"
    apply   = "Live"
  }

  field "keys.delete_point" {
    label   = "Delete path point"
    kind    = "chord"
    default = "F3+Delete"
    apply   = "Live"
  }

  field "keys.fly_forward" {
    label   = "Fly forward"
    kind    = "chord"
    default = "W"
    apply   = "Live"
  }

  field "keys.fly_back" {
    label   = "Fly back"
    kind    = "chord"
    default = "S"
    apply   = "Live"
  }

  field "keys.fly_left" {
    label   = "Fly left"
    kind    = "chord"
    default = "A"
    apply   = "Live"
  }

  field "keys.fly_right" {
    label   = "Fly right"
    kind    = "chord"
    default = "D"
    apply   = "Live"
  }

  field "keys.fly_up" {
    label   = "Fly up"
    kind    = "chord"
    default = "Space"
    apply   = "Live"
  }

  field "keys.fly_down" {
    label   = "Fly down"
    kind    = "chord"
    default = "Shift"
    apply   = "Live"
  }

  field "keys.fly_fast" {
    label   = "Fly fast"
    kind    = "chord"
    default = "Ctrl"
    apply   = "Live"
  }
}

section "motion" {
  title = "Motion"

  field "motion.start_speed" {
    label   = "Start speed"
    kind    = { float = { min = 0.5, max = 80.0, step = 0.5, unit = "blocks/s" } }
    default = 6.0
    apply   = "Live"
  }

  field "motion.min_speed" {
    label   = "Lowest speed"
    kind    = { float = { min = 0.5, max = 80.0, step = 0.5, unit = "blocks/s" } }
    default = 0.5
    apply   = "Live"
  }

  field "motion.max_speed" {
    label   = "Highest speed"
    kind    = { float = { min = 0.5, max = 400.0, step = 0.5, unit = "blocks/s" } }
    default = 80.0
    apply   = "Live"
  }

  field "motion.scroll_factor" {
    label   = "Speed change per wheel notch"
    kind    = { float = { min = 1.0, max = 3.0, step = 0.05 } }
    default = 1.25
    apply   = "Live"
  }

  field "motion.fast_multiplier" {
    label   = "Fast speed multiplier"
    kind    = { float = { min = 1.0, max = 10.0, step = 0.5 } }
    default = 3.0
    apply   = "Live"
  }

  field "motion.sensitivity" {
    label   = "Mouse sensitivity"
    kind    = { float = { min = 0.01, max = 1.0, step = 0.01, unit = "deg/count" } }
    default = 0.15
    apply   = "Live"
  }

  field "motion.move_ease_rate" {
    label   = "Movement easing"
    help    = "Higher is snappier; 0 turns easing off"
    kind    = { float = { min = 0.0, max = 30.0, step = 0.5 } }
    default = 6.0
    apply   = "Live"
  }

  field "motion.look_ease_rate" {
    label   = "Look easing"
    help    = "Higher is snappier; 0 turns easing off"
    kind    = { float = { min = 0.0, max = 30.0, step = 0.5 } }
    default = 14.0
    apply   = "Live"
  }

  field "motion.pitch_limit" {
    label   = "Pitch limit"
    kind    = { float = { min = 1.0, max = 89.9, step = 0.1, unit = "deg" } }
    default = 89.9
    apply   = "Live"
  }

  field "motion.max_frame_dt" {
    label   = "Longest frame step"
    kind    = { float = { min = 0.01, max = 1.0, step = 0.01, unit = "s" } }
    default = 0.1
    apply   = "Live"
  }

  field "motion.smooth_at_start" {
    label   = "Smooth camera at start"
    kind    = "bool"
    default = true
    apply   = "Live"
  }
}

section "markers" {
  title = "Path markers"

  field "markers.enabled" {
    label   = "Show path markers"
    kind    = "bool"
    default = true
    apply   = "Restart"
  }

  field "markers.max_shapes" {
    label   = "Shape budget"
    kind    = { int = { min = 2560, max = 8192, step = 64 } }
    default = 4096
    apply   = "Restart"
  }

  field "markers.max_per_send" {
    label   = "Shapes sent per frame"
    kind    = { int = { min = 50, max = 800, step = 10 } }
    default = 400
    apply   = "Restart"
  }

  field "markers.line_weight" {
    label   = "Line weight"
    kind    = { int = { min = 1, max = 2, step = 1 } }
    default = 2
    apply   = "Restart"
  }

  field "markers.frustum_depth" {
    label   = "Camera box depth"
    kind    = { float = { min = 0.3, max = 5.0, step = 0.1, unit = "blocks" } }
    default = 1.2
    apply   = "Restart"
  }
}

section "look_at" {
  title = "Look-at"

  field "look_at.enabled" {
    label   = "Add look-at points"
    help    = "Adding a point also adds a look-at key when the crosshair hits a block"
    kind    = "bool"
    default = true
    apply   = "Live"
  }

  field "look_at.max_distance" {
    label   = "Ray length"
    kind    = { float = { min = 8.0, max = 512.0, step = 8.0, unit = "blocks" } }
    default = 256.0
    apply   = "Live"
  }
}

section "advanced" {
  title = "Advanced"

  field "exit_wait_frames" {
    label   = "Exit wait frames"
    kind    = { int = { min = 4, max = 120, step = 1 } }
    default = 10
    apply   = "Restart"
  }

  field "telemetry" {
    label   = "Telemetry log"
    help    = "Logs frame pacing and key-to-move times while flying"
    kind    = "bool"
    default = false
    apply   = "Restart"
  }
}
