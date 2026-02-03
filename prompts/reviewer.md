Please dispatch the implementation manager agent with the following prompt:

"Study the current code. Then look at docs/issues/open, and decide which one needs to be implemented next. Launch an agent to implement this issue - ensure that the 'cargo build' and 'cargo test' pass, and then commit. IMPORTANT: instruct to always do 'cargo build' and 'cargo test' at top level, rather than crates! After it is done, launch verifier agent to verify that the issue has been implemented correctly. Repeat until the issue is implemented correctly, according to verifier agent. Move on to the next issue. Repeat until you run out of issues. If something is missing or wrong, an issue MUST be created in accordance to docs/issues/README.md - ensure this is communicate do implementer and verifier agents. IMPORTANT: before moving on to the new issue, ensure the current outstanding changes are committed."

Afterwards, dispatch the implementation improver agent with the following prompt:

"
Study the current code. Then look at docs/issues/resolved, and verify that the code as is does take care of them - if not, then create a new issue referencing the old one that is supposedly "resolved", and put it into docs/issues/open. Use the same general format - detailed, with the suggested solution where the low-skilled developer can implement the solution. Then perform the general audit of the code and its conformance to the plans - and create issues as needed, if something does not conform. 

All the tests should pass. There should be no warnings when compiling (fix them by pointed commits).
It should be possible to compile and run the apchat, use the /-commands, check that the commands can be confirmed, etc. This can be tested using pty tools.
If something prevents this - open an issue. Open as many issues as possible, be very deep in your analysis. Uberthink it.
If you run out of things to do, explore the codebase, find patterns, and create memories to capture them, analyze the existing memories as well.
Improve code base. Uberthink it.
"
