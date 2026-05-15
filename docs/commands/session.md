# farscry session

List and inspect recorded sessions.

## Commands

```bash
farscry session --list
farscry session --latest
```

## Output format

```
20260515-143022-12345.vasf   2m 34s   154 frames   12 unique   92% dedup   47KB
```

## Analyze a session

```bash
farscry timeline ~/.farscry/sessions/20260515-143022-12345.vasf
farscry info ~/.farscry/sessions/20260515-143022-12345.vasf
```
