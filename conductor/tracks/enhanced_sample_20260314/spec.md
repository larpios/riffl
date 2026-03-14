# Track Specification: Enhanced Sample Management and ProTracker Effects

## Overview
This track aims to enhance the audio engine of Riffl by adding support for common sample-level parameters (Fine-tune, Loops, Volume) and implementing classic ProTracker effects. These additions are crucial for achieving the depth of a professional music tracker like Renoise.

## Goals
- Support for sample fine-tuning.
- Support for sample loop points (bidirectional, forward, none).
- Refined sample volume control.
- Implementation of basic ProTracker effects (Arpeggio, Portamento, Vibrato, etc.).
- Update UI to reflect and edit these new parameters.

## Technical Requirements
- Audio engine must process fine-tune and loop data in real-time.
- ProTracker effect logic must be integrated into the pattern playback system.
- TUI must provide intuitive controls for managing sample parameters and effect commands.

## Out of Scope
- Advanced synthesis (VST/AU plugins).
- Advanced MIDI integration (outside of basic pattern triggers).
- Multi-track mixing desk (full channel strip).
