"""프레임들을 나란히 붙인 시트와 GIF 를 만든다."""

import argparse
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("-o", "--out", type=Path, required=True)
    args = parser.parse_args()
    # TODO
    raise NotImplementedError


if __name__ == "__main__":
    main()
