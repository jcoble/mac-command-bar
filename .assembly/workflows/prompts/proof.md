Task ${{ task.key }}: ${{ task.title }}

${{ task.body }}

What was built:
${{ steps.build.summary }}

Prove the change works. For a visible change, run the app headless as the workbox-native-ui skill describes, show the change, and save screenshots or a short video in ${{ run.proof_dir }}; describe each file with `evidence add <file> --task ${{ task.key }} --desc "<what it proves>"`. For a change you cannot see, save readable command receipts there instead. Stop every process you started.

Report: each proof file and what it shows, and each command with its result. Never claim a result you did not see.
