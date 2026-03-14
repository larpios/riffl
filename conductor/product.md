# Initial Concept
Tracker-rs is an ambitious TUI music tracker app with coding capabilities like in Strudel written in Rust.

# Product Guide: Tracker-rs

## Product Vision
Tracker-rs is an ambitious music tracker app, primarily focusing on a terminal-based interface (TUI) with future plans for a graphical user interface (GUI). Inspired by the depth of Renoise and the creative coding capabilities of Strudel, Tracker-rs aims to bridge the gap between traditional step-based tracking and modern algorithmic composition.

## Core Philosophy: Hybrid Workflow
Tracker-rs embraces a **Hybrid Workflow**, where the user interface and a robust domain-specific language (DSL) for scripting and automation coexist. This allows for both precise manual editing of patterns and dynamic, code-driven generation of musical structures.

## Target Audience
- **Tracker Enthusiasts:** Musicians who demand the speed and efficiency of a keyboard-driven interface.
- **Live Coders:** Performers looking for a portable, stable, and scriptable environment for generative music.
- **Developers:** Users who want to integrate code-based logic and automation into their creative process.

## Key Functionality & Roadmap
1.  **TUI & Future GUI:** Build a professional-grade TUI capable of handling complex pattern editing, instrument management, and visual representation of audio data, while maintaining a flexible architecture for future GUI development.
2.  **Scripting & Automation:** Implement a powerful coding capability (inspired by Strudel) for algorithmic pattern generation and parameter automation. (While Rhai is currently used, the architecture should remain open to potential DSL improvements).
3.  **Sound Design Tools:** Integrate high-quality audio effects, mixing capabilities, and robust sample management.
4.  **Compatibility & Integration:** Support for standard tracker formats and MIDI to ensure interoperability with existing workflows and hardware.
5.  **Multi-Genre Potential:** Provide a flexible architecture capable of producing any genre of music, from electronic and chip-tune to more experimental and avant-garde styles.
6.  **Production Ready:** The ability to export high-fidelity, professional-sounding tracks directly from the app.

## Success Criteria
- **Rock-Solid Audio:** Zero glitches and consistent low-latency performance across all supported platforms.
- **Intuitive UX:** A logical and streamlined interface that empowers the user without getting in the way.
- **Stability:** High-performance core that remains stable under heavy system load.
