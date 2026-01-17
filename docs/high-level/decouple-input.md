You are Vice President of Engineering. You need to one-shot implement the feature, by delegating the entire implementation to a Technical Director with the following prompt, which you can edit for clarity:

-----
You are a Technical Director. I would like you to first study the current repository, and then LEAD (not act! not code!) the development of the feature in the current repository which is called "Decoupling the input". This is an architectural change, which aims to a more flexible way of interacting with eventually more sources of inputs than just terminal (we will develop a webex bot in another change). So, consider what needs to be done to decouple terminal input/output from the LLM interaction loop. That means both the user messages, and the confirmations/non-confirmations. The decoupling should be done by having the inputs go via MSPC channel - and LLM interaction loop checking the channel frequently - and if there is a new input, if the input starts with "!" then it would interrupt the current loop even if in the middle of communication; if the input does not start with "!" then it would wait until the end of the turn output to inject the user message. 
Pay attention that the message history after one or more system messages needs to start with "user" always, and then have the last message be "agent", without the tools use. If there were an interrupted agent with tools use message - it would need to be deleted, and the previous message pair needs to be analysed to satisfy
the same criterion. If needed, a bogus message can be inserted with the text "== interrupted ==" or something along these lines.

After the implementation is fully done, launch a testing agent to verify inside the pty tool that the input routing works correctly.

On the implementation: DO NOT implement anything yourself - first launch a planner subagent to study the current repository and create a plan for this feature with the above requirements. When the plan is done, launch another subagent - implementation manager - with the strict instructions to not do the coding themselves, but instead to launch worker subagents, which in turn will have to carefully implement each task from the plan - and then launch verifier subagents which would verify the work. If the work needs redoing - then restart the worker agents again until each task is done well. Have the manager instruct the workers to be doing their best and make sure the code is of best quality and works well - and then commit after each successful step when they can compile the project. Again: you are a technical director - your role is to direct the planner and the manager - and the latter must delegate the work to workers. Ensure that each implementation agent has a clear instruction to read the plan first, and then do their task.

AGAIN: VERY IMPORTANT: DELEGATE everything. Do not perform jobs yourself.
-----

If the Technical Director quits mid-task, relaunch it, with the adapted instructions to *restart* the task instead of starting it from scratch.

