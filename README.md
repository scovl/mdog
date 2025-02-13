# MDog - Keyboard/Mouse to Xbox Controller Converter

MDog is a powerful tool that converts keyboard and mouse inputs into Xbox 360 controller commands, allowing you to play controller-based games with keyboard and mouse.

## Features

- Convert keyboard and mouse inputs to Xbox 360 controller inputs
- Customizable key bindings
- Mouse movement to analog stick conversion
- Adjustable mouse sensitivity
- Special "parachute mode" for precision control (toggle with X key)
- Mouse smoothing options
- Real-time input switching

## Requirements

- Windows OS
- Administrator privileges (required for input interception)
- ViGEmBus driver (automatically installed during setup)

## Installation

1. Download the latest release
2. Run the installer as administrator
3. Follow the setup instructions

## Configuration

The program uses a RON (Rusty Object Notation) configuration file that allows you to:
- Customize key bindings
- Adjust mouse sensitivity
- Set mouse smoothing level
- Configure special keys

## Usage

1. Launch the program as administrator
2. Use the toggle key (default: ` [grave/tilde]) to enable/disable the converter
3. Press X to toggle parachute mode for precision control

## Technical Details

Built with:
- Rust
- Interception Driver (for input capture)
- ViGEm (for Xbox controller emulation)

