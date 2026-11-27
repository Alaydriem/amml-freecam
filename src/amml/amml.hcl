namespace   = "alaydriem"
name        = "freecam"
entry       = "alaydriem_freecam.dll"
api_version = 3

uses {
  feature "flight" {
    event "client.tick" {
      version  = 1
      required = true
    }
    event "camera.setup" {
      version  = 1
      required = true
    }
    event "camera.instruction_received" {
      version  = 1
      required = true
    }
    event "input.key" {
      version  = 1
      required = true
    }
    event "input.mouse" {
      version  = 1
      required = true
    }
    event "world.left" {
      version  = 1
      required = true
    }
    event "window.focus" {
      version  = 1
      required = true
    }
    event "registry.provider_ready" {
      version  = 1
      required = true
    }
    event "registry.provider_gone" {
      version  = 1
      required = true
    }
    event "mods.loaded" {
      version  = 1
      required = true
    }
    event "input.lease_suspended" {
      version  = 1
      required = true
    }
    event "input.lease_resumed" {
      version  = 1
      required = true
    }
    event "input.lease_lost" {
      version  = 1
      required = true
    }
    event "input.injected_released" {
      version  = 2
      required = true
    }
    area "client.read" {
      version  = 1
      required = true
    }
    area "input.lease" {
      version  = 1
      required = true
    }
    area "actor.read" {
      version  = 3
      required = true
    }
    area "actor.health" {
      version  = 1
      required = true
    }
    area "camera.pose" {
      version  = 1
      required = true
    }
    area "camera.projection" {
      version  = 1
      required = true
    }
    area "camera.free" {
      version  = 1
      required = true
    }
  }
  feature "markers" {
    area "world.shapes" {
      version  = 1
      required = true
    }
  }
  feature "camera_watch" {
    area "camera.active" {
      version  = 1
      required = true
    }
  }
  feature "look_at" {
    area "world.raycast" {
      version  = 1
      required = true
    }
    area "world.actors_in" {
      version  = 1
      required = true
    }
  }
  feature "screenshots" {
    event "overlay.screenshot_taken" {
      version  = 1
      required = true
    }
    event "overlay.screenshot_written" {
      version  = 1
      required = true
    }
    area "overlay.screenshot" {
      version  = 1
      required = true
    }
  }
}

listing {
  title       = "Freecam"
  summary     = "Free camera with keyframed camera paths for recording smooth shots."
  description = <<-EOT
    A free camera mod to look at builds and scout out areas independently of your player position. It also has basic cinematic tools: create simple keyframe paths and quickly take screenshots.
  EOT
  authors     = ["Charles R. Portwood II"]
  license     = "BSD-3-Clause"
  source      = "https://github.com/alaydriem/amml-freecam"
  homepage    = "https://github.com/alaydriem/amml-freecam#readme"
  tags        = ["freecam", "camera", "cinematic", "recording", "client"]
  icon        = "icon.png"
}
