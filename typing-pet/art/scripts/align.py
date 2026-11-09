"""각 프레임의 책상 영역 위치를 base.png 기준으로 맞춘다."""

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
