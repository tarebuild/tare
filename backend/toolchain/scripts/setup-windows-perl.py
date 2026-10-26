"""Install checksum-pinned portable Perl locally for stock Windows IDL builds."""
from pathlib import Path
import hashlib
import urllib.request
import zipfile

URL = "https://github.com/StrawberryPerl/Perl-Dist-Strawberry/releases/download/SP_54231_64bit/strawberry-perl-5.42.3.1-64bit-portable.zip"
SHA256 = "6a081a811781c30aca51dbc036afd93092af91e3297901f02c17043795a10690"
root = Path(__file__).resolve().parents[1] / "host" / "strawberry-perl"
archive_path = root.parent / "strawberry-perl.zip"
root.mkdir(parents=True, exist_ok=True)
if not archive_path.exists():
    urllib.request.urlretrieve(URL, archive_path)
with archive_path.open("rb") as stream:
    digest = hashlib.file_digest(stream, "sha256").hexdigest()
if digest != SHA256:
    raise SystemExit("Portable Perl archive checksum mismatch")
with zipfile.ZipFile(archive_path) as archive:
    for member in archive.infolist():
        target = (root / member.filename).resolve()
        if not target.is_relative_to(root.resolve()):
            raise SystemExit("Unsafe archive path")
    archive.extractall(root)
print(f'$env:OPENSSL_SRC_PERL = "{root / "perl/bin/perl.exe"}"')
print('Remove-Item Env:PERL5LIB -ErrorAction SilentlyContinue')
