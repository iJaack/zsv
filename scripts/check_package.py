"""Bounded publication checks, not a credential scanner or security audit."""
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
files = [p for p in root.rglob('*')
         if '.git' not in p.relative_to(root).parts
         and 'target' not in p.relative_to(root).parts and p.is_file()]
issues = []
patterns = {
    'GitHub credential': re.compile(r'gh[pousr]_[A-Za-z0-9]{20,}'),
    'AWS access key': re.compile(r'\b(?:AKIA|ASIA)[A-Z0-9]{16}\b'),
    'private-key block': re.compile(r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'),
}
for path in files:
    relative = path.relative_to(root)
    if path.is_symlink():
        issues.append(f'{relative}: symlink excluded from publication package')
        continue
    if path.name.startswith('.env') or path.suffix in {'.pem', '.key'}:
        issues.append(f'{relative}: private configuration/key file')
    if path.suffix in {'.pdf', '.png'}:
        continue
    try:
        text = path.read_text()
    except UnicodeDecodeError:
        issues.append(f'{relative}: unexpected binary file')
        continue
    for label, pattern in patterns.items():
        if pattern.search(text):
            issues.append(f'{relative}: possible {label} (value suppressed)')
    if path.suffix == '.md':
        for target in re.findall(r'\]\(([^)]+)\)', text):
            if '://' in target or target.startswith(('#', 'mailto:')):
                continue
            target = target.split('#', 1)[0]
            if target and not (path.parent / target).exists():
                issues.append(f'{relative}: missing relative link {target}')
if issues:
    print('\n'.join(issues))
    sys.exit(1)
print(f'Package check passed: {len(files)} files; links and bounded credential patterns.')
