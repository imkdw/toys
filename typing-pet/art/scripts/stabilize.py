"""base 와 변형 프레임의 차이 마스크를 구해 차이 나는 부분(팔, 얼굴)만 base 위에 덮어쓴다."""

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
