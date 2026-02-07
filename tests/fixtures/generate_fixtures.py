#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path


def make_pdf(path: Path, pages: int) -> None:
    objects: list[str | None] = []

    def add(obj: str | None) -> int:
        objects.append(obj)
        return len(objects)

    font_id = add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")

    content_ids: list[int] = []
    page_ids: list[int] = []

    for i in range(pages):
        text = f"BT /F1 24 Tf 72 720 Td (ButterPaper fixture page {i + 1}) Tj ET"
        stream = f"<< /Length {len(text.encode('utf-8'))} >>\nstream\n{text}\nendstream"
        content_ids.append(add(stream))
        page_ids.append(add(None))

    kids = " ".join(f"{pid} 0 R" for pid in page_ids)
    pages_id = add(f"<< /Type /Pages /Kids [{kids}] /Count {pages} >>")
    catalog_id = add(f"<< /Type /Catalog /Pages {pages_id} 0 R >>")

    for index, page_id in enumerate(page_ids):
        objects[page_id - 1] = (
            "<< /Type /Page "
            f"/Parent {pages_id} 0 R "
            "/MediaBox [0 0 612 792] "
            f"/Contents {content_ids[index]} 0 R "
            f"/Resources << /Font << /F1 {font_id} 0 R >> >> >>"
        )

    assert all(obj is not None for obj in objects)

    output = bytearray()
    output.extend(b"%PDF-1.4\n")
    output.extend(b"%\xe2\xe3\xcf\xd3\n")

    offsets = [0]

    for obj_number, obj in enumerate(objects, start=1):
        offsets.append(len(output))
        output.extend(f"{obj_number} 0 obj\n".encode("ascii"))
        output.extend((obj or "").encode("utf-8"))
        output.extend(b"\nendobj\n")

    xref_offset = len(output)
    output.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    output.extend(b"0000000000 65535 f \n")

    for offset in offsets[1:]:
        output.extend(f"{offset:010} 00000 n \n".encode("ascii"))

    output.extend(b"trailer\n")
    output.extend(f"<< /Size {len(objects) + 1} /Root {catalog_id} 0 R >>\n".encode("ascii"))
    output.extend(b"startxref\n")
    output.extend(f"{xref_offset}\n".encode("ascii"))
    output.extend(b"%%EOF\n")

    path.write_bytes(output)


def main() -> None:
    root = Path(__file__).resolve().parent
    make_pdf(root / "small.pdf", pages=1)
    make_pdf(root / "medium.pdf", pages=5)
    make_pdf(root / "large.pdf", pages=20)

    (root / "invalid.pdf").write_text("this is not a pdf\n", encoding="utf-8")
    (root / "encrypted-marker.pdf").write_bytes(b"%PDF-1.4\n1 0 obj\n<< /Encrypt true >>\nendobj\n%%EOF\n")


if __name__ == "__main__":
    main()
