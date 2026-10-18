# OmniRush integration idea

This note captures a possible future direction; it is not part of the current Tickt product.

Tickt could inspire an optional built-in focus mode for OmniRush. A terminal user working in an
OmniRush session might be able to run a command such as `omnirush focus`, keep a Pomodoro timer
beside the active task, and receive a local summary when the session ends. A thoughtful version
could connect focus sessions to the current local workspace while preserving Tickt's principles:

- local-first storage;
- no telemetry or required cloud service;
- keyboard-driven terminal interaction;
- explicit user control over what task metadata is recorded;
- recoverable timers and conservative completion accounting.

Possible implementation directions for OmniRush would be an optional built-in subcommand or a
small shared terminal UI component. Tickt is deliberately not coupled to either approach today:
it has no OmniRush dependency, performs no integration calls, and remains useful as an independent
Linux terminal application.
