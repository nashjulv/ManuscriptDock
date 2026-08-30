#!/usr/bin/env python3
"""Build the branded ManuscriptDock product manual PDF from Markdown."""

from __future__ import annotations

import re
from pathlib import Path
from xml.sax.saxutils import escape

from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    BaseDocTemplate,
    Flowable,
    Frame,
    Image,
    ListFlowable,
    ListItem,
    LongTable,
    PageBreak,
    PageTemplate,
    Paragraph,
    Preformatted,
    Spacer,
    Table,
    TableStyle,
)
from reportlab.platypus.tableofcontents import TableOfContents


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "docs" / "user-manual.md"
LOGO = ROOT / "apps" / "desktop" / "src-tauri" / "icons" / "icon.png"
OUTPUT = ROOT / "output" / "pdf" / "manuscriptdock-user-manual.pdf"

PAGE_WIDTH, PAGE_HEIGHT = A4
LEFT = 20 * mm
RIGHT = 20 * mm
TOP = 20 * mm
BOTTOM = 18 * mm
CONTENT_WIDTH = PAGE_WIDTH - LEFT - RIGHT

GREEN = colors.HexColor("#A6CE39")
GREEN_DARK = colors.HexColor("#557315")
INK = colors.HexColor("#343230")
TEXT = colors.HexColor("#4A4846")
MUTED = colors.HexColor("#77736F")
LINE = colors.HexColor("#D9D7D3")
PALE = colors.HexColor("#F3F4F0")
SOFT_GREEN = colors.HexColor("#F2F7E5")

PROCESS_STAGES = ["导入", "检查", "修订", "版本", "期刊匹配", "存证", "投稿", "知识体"]


def register_fonts() -> None:
    pdfmetrics.registerFont(
        TTFont("ManualSans", "/System/Library/Fonts/STHeiti Light.ttc", subfontIndex=0)
    )
    pdfmetrics.registerFont(
        TTFont("ManualSans-Medium", "/System/Library/Fonts/STHeiti Medium.ttc", subfontIndex=0)
    )
    pdfmetrics.registerFontFamily(
        "ManualSans",
        normal="ManualSans",
        bold="ManualSans-Medium",
        italic="ManualSans",
        boldItalic="ManualSans-Medium",
    )


def build_styles() -> dict[str, ParagraphStyle]:
    base = getSampleStyleSheet()
    return {
        "CoverTitle": ParagraphStyle(
            "CoverTitle",
            parent=base["Title"],
            fontName="ManualSans-Medium",
            fontSize=27,
            leading=36,
            textColor=INK,
            alignment=TA_LEFT,
            spaceAfter=6 * mm,
            wordWrap="CJK",
        ),
        "CoverEnglish": ParagraphStyle(
            "CoverEnglish",
            fontName="ManualSans",
            fontSize=12,
            leading=18,
            textColor=MUTED,
            spaceAfter=18 * mm,
            wordWrap="CJK",
        ),
        "CoverSlogan": ParagraphStyle(
            "CoverSlogan",
            fontName="ManualSans-Medium",
            fontSize=16,
            leading=24,
            textColor=INK,
            spaceAfter=2 * mm,
            wordWrap="CJK",
        ),
        "CoverSloganEn": ParagraphStyle(
            "CoverSloganEn",
            fontName="ManualSans",
            fontSize=10,
            leading=16,
            textColor=MUTED,
            spaceAfter=18 * mm,
        ),
        "CoverMeta": ParagraphStyle(
            "CoverMeta",
            fontName="ManualSans",
            fontSize=9.2,
            leading=16,
            textColor=MUTED,
            wordWrap="CJK",
        ),
        "TocTitle": ParagraphStyle(
            "TocTitle",
            fontName="ManualSans-Medium",
            fontSize=22,
            leading=30,
            textColor=INK,
            spaceAfter=8 * mm,
        ),
        "SectionHeading": ParagraphStyle(
            "SectionHeading",
            fontName="ManualSans-Medium",
            fontSize=16.5,
            leading=24,
            textColor=INK,
            spaceBefore=8 * mm,
            spaceAfter=4 * mm,
            keepWithNext=True,
            wordWrap="CJK",
        ),
        "SectionHeadingCompact": ParagraphStyle(
            "SectionHeadingCompact",
            fontName="ManualSans-Medium",
            fontSize=16.5,
            leading=24,
            textColor=INK,
            spaceBefore=2 * mm,
            spaceAfter=1.5 * mm,
            keepWithNext=True,
            wordWrap="CJK",
        ),
        "Subheading": ParagraphStyle(
            "Subheading",
            fontName="ManualSans-Medium",
            fontSize=11.5,
            leading=18,
            textColor=INK,
            spaceBefore=5 * mm,
            spaceAfter=2.5 * mm,
            keepWithNext=True,
            wordWrap="CJK",
        ),
        "Body": ParagraphStyle(
            "Body",
            fontName="ManualSans",
            fontSize=9.4,
            leading=16.2,
            textColor=TEXT,
            spaceAfter=2.6 * mm,
            wordWrap="CJK",
            allowWidows=0,
            allowOrphans=0,
        ),
        "Bullet": ParagraphStyle(
            "Bullet",
            fontName="ManualSans",
            fontSize=9.3,
            leading=15.5,
            textColor=TEXT,
            leftIndent=5 * mm,
            firstLineIndent=0,
            bulletIndent=0,
            spaceAfter=0.65 * mm,
            wordWrap="CJK",
        ),
        "Numbered": ParagraphStyle(
            "Numbered",
            fontName="ManualSans",
            fontSize=9.3,
            leading=15.5,
            textColor=TEXT,
            leftIndent=7 * mm,
            firstLineIndent=0,
            bulletIndent=0,
            spaceAfter=1.2 * mm,
            wordWrap="CJK",
        ),
        "Code": ParagraphStyle(
            "Code",
            fontName="Courier",
            fontSize=8.5,
            leading=13,
            textColor=INK,
            leftIndent=4 * mm,
            rightIndent=4 * mm,
            borderColor=LINE,
            borderWidth=0.6,
            borderPadding=4 * mm,
            backColor=PALE,
            spaceBefore=2 * mm,
            spaceAfter=4 * mm,
        ),
        "TableHeader": ParagraphStyle(
            "TableHeader",
            fontName="ManualSans-Medium",
            fontSize=8.5,
            leading=13,
            textColor=INK,
            wordWrap="CJK",
        ),
        "TableBody": ParagraphStyle(
            "TableBody",
            fontName="ManualSans",
            fontSize=8.2,
            leading=13,
            textColor=TEXT,
            wordWrap="CJK",
        ),
        "Process": ParagraphStyle(
            "Process",
            fontName="ManualSans-Medium",
            fontSize=8.2,
            leading=13,
            textColor=INK,
            alignment=1,
            wordWrap="CJK",
        ),
        "Callout": ParagraphStyle(
            "Callout",
            fontName="ManualSans",
            fontSize=9.2,
            leading=16,
            textColor=TEXT,
            leftIndent=4 * mm,
            rightIndent=4 * mm,
            borderColor=GREEN,
            borderWidth=0,
            borderLeft=True,
            borderPadding=3 * mm,
            backColor=SOFT_GREEN,
            spaceBefore=2 * mm,
            spaceAfter=4 * mm,
            wordWrap="CJK",
        ),
    }


def inline_markup(text: str) -> str:
    value = escape(text.strip()).replace("→", "&gt;")
    value = value.replace("&lt;br&gt;", "<br/>")
    value = re.sub(
        r"\[([^\]]+)\]\(([^)]+)\)",
        lambda match: (
            f'<link href="{match.group(2)}" color="#557315"><u>{match.group(1)}</u></link>'
            if match.group(2).startswith(("https://", "http://"))
            else f"<u>{match.group(1)}</u>"
        ),
        value,
    )
    value = re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", value)
    value = re.sub(r"`([^`]+)`", r'<font name="Courier" color="#343230">\1</font>', value)
    value = re.sub(r"(?<!\*)\*([^*]+)\*(?!\*)", r"<i>\1</i>", value)
    return value


class ManualDocTemplate(BaseDocTemplate):
    def __init__(self, filename: str, **kwargs):
        super().__init__(filename, **kwargs)
        frame = Frame(
            LEFT,
            BOTTOM,
            CONTENT_WIDTH,
            PAGE_HEIGHT - TOP - BOTTOM,
            leftPadding=0,
            rightPadding=0,
            topPadding=8 * mm,
            bottomPadding=4 * mm,
            id="manual-frame",
        )
        self.addPageTemplates(PageTemplate(id="manual", frames=[frame], onPage=self.draw_page))

    def draw_page(self, canvas, doc) -> None:
        canvas.saveState()
        if doc.page > 1:
            canvas.setStrokeColor(GREEN)
            canvas.setLineWidth(1.5)
            canvas.line(LEFT, PAGE_HEIGHT - 13 * mm, PAGE_WIDTH - RIGHT, PAGE_HEIGHT - 13 * mm)
            canvas.setFont("ManualSans-Medium", 7.8)
            canvas.setFillColor(MUTED)
            canvas.drawString(LEFT, PAGE_HEIGHT - 10 * mm, "投稿舱 ManuscriptDock · 产品使用手册")
            canvas.setFont("ManualSans", 7.6)
            canvas.drawRightString(PAGE_WIDTH - RIGHT, 9 * mm, f"{doc.page}")
            canvas.setStrokeColor(LINE)
            canvas.setLineWidth(0.5)
            canvas.line(LEFT, 13 * mm, PAGE_WIDTH - RIGHT, 13 * mm)
        canvas.restoreState()

    def afterFlowable(self, flowable) -> None:
        if not isinstance(flowable, Paragraph):
            return
        if flowable.style.name not in {"SectionHeading", "SectionHeadingCompact", "Subheading"}:
            return
        level = 0 if flowable.style.name.startswith("SectionHeading") else 1
        text = flowable.getPlainText()
        key = f"heading-{self.seq.nextf('heading')}"
        self.canv.bookmarkPage(key)
        self.canv.addOutlineEntry(text, key, level=level, closed=False)
        self.notify("TOCEntry", (level, text, self.page, key))


def cover_story(styles: dict[str, ParagraphStyle]) -> list[Flowable]:
    privacy = Table(
        [[Paragraph("<b>本地优先</b> · 稿件默认留在设备上；模型调用逐次授权并在 Rust 出口脱敏。", styles["Body"])]],
        colWidths=[CONTENT_WIDTH],
    )
    privacy.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, -1), SOFT_GREEN),
                ("BOX", (0, 0), (-1, -1), 0.8, GREEN),
                ("LEFTPADDING", (0, 0), (-1, -1), 5 * mm),
                ("RIGHTPADDING", (0, 0), (-1, -1), 5 * mm),
                ("TOPPADDING", (0, 0), (-1, -1), 4 * mm),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 4 * mm),
            ]
        )
    )
    logo = Image(str(LOGO), width=38 * mm, height=38 * mm)
    logo.hAlign = "LEFT"
    return [
        Spacer(1, 13 * mm),
        logo,
        Spacer(1, 7 * mm),
        Paragraph("投稿舱 ManuscriptDock", styles["CoverTitle"]),
        Paragraph("产品使用手册", styles["CoverTitle"]),
        Paragraph("Local-first manuscript submission workspace", styles["CoverEnglish"]),
        Paragraph("投论文，上更好的期刊", styles["CoverSlogan"]),
        Paragraph("Go for Better Journals.", styles["CoverSloganEn"]),
        privacy,
        Spacer(1, 18 * mm),
        Paragraph(
            "版本 0.1.0 开发 MVP<br/>适用：macOS 11+ · Windows 10/11 x64<br/>手册日期：2026-08-30",
            styles["CoverMeta"],
        ),
        PageBreak(),
    ]


def toc_story(styles: dict[str, ParagraphStyle]) -> list[Flowable]:
    toc = TableOfContents()
    toc.levelStyles = [
        ParagraphStyle(
            "TOC1",
            fontName="ManualSans-Medium",
            fontSize=9.2,
            leading=16,
            leftIndent=0,
            firstLineIndent=0,
            textColor=INK,
            spaceBefore=2,
        ),
        ParagraphStyle(
            "TOC2",
            fontName="ManualSans",
            fontSize=8.2,
            leading=14,
            leftIndent=6 * mm,
            firstLineIndent=0,
            textColor=MUTED,
        ),
    ]
    return [
        Paragraph("目录", styles["TocTitle"]),
        Paragraph("按完整投稿流程组织，章节可在 PDF 书签中直接跳转。", styles["Body"]),
        Spacer(1, 3 * mm),
        toc,
        PageBreak(),
    ]


def make_table(rows: list[list[str]], styles: dict[str, ParagraphStyle]) -> LongTable:
    columns = max(len(row) for row in rows)
    normalized = [row + [""] * (columns - len(row)) for row in rows]
    data = []
    for row_index, row in enumerate(normalized):
        style = styles["TableHeader"] if row_index == 0 else styles["TableBody"]
        data.append([Paragraph(inline_markup(cell), style) for cell in row])
    if columns == 2:
        widths = [CONTENT_WIDTH * 0.31, CONTENT_WIDTH * 0.69]
    elif columns == 3:
        widths = [CONTENT_WIDTH * 0.24, CONTENT_WIDTH * 0.18, CONTENT_WIDTH * 0.58]
    else:
        widths = [CONTENT_WIDTH / columns] * columns
    table = LongTable(data, colWidths=widths, repeatRows=1, splitByRow=1)
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, 0), PALE),
                ("TEXTCOLOR", (0, 0), (-1, 0), INK),
                ("GRID", (0, 0), (-1, -1), 0.45, LINE),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 3 * mm),
                ("RIGHTPADDING", (0, 0), (-1, -1), 3 * mm),
                ("TOPPADDING", (0, 0), (-1, -1), 2.4 * mm),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 2.4 * mm),
            ]
        )
    )
    return table


def make_process_flow(styles: dict[str, ParagraphStyle]) -> Table:
    cells = [
        Paragraph(f'<font color="#557315">{index:02d}</font><br/><b>{stage}</b>', styles["Process"])
        for index, stage in enumerate(PROCESS_STAGES, 1)
    ]
    table = Table([cells], colWidths=[CONTENT_WIDTH / len(cells)] * len(cells))
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, -1), SOFT_GREEN),
                ("BOX", (0, 0), (-1, -1), 0.8, GREEN),
                ("INNERGRID", (0, 0), (-1, -1), 0.45, colors.white),
                ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                ("TOPPADDING", (0, 0), (-1, -1), 2.5 * mm),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 2.5 * mm),
                ("LEFTPADDING", (0, 0), (-1, -1), 1 * mm),
                ("RIGHTPADDING", (0, 0), (-1, -1), 1 * mm),
            ]
        )
    )
    return table


def parse_markdown(styles: dict[str, ParagraphStyle]) -> list[Flowable]:
    lines = SOURCE.read_text(encoding="utf-8").splitlines()
    start = next(index for index, line in enumerate(lines) if line.startswith("## "))
    lines = lines[start:]
    story: list[Flowable] = []
    index = 0
    while index < len(lines):
        raw = lines[index]
        stripped = raw.strip()
        if not stripped:
            index += 1
            continue
        if stripped == "导入 → 检查 → 修订 → 版本 → 期刊匹配 → 存证 → 投稿 → 知识体":
            story.extend([Spacer(1, 1 * mm), make_process_flow(styles), Spacer(1, 4 * mm)])
            index += 1
            continue
        if stripped.startswith("```"):
            code_lines = []
            index += 1
            while index < len(lines) and not lines[index].strip().startswith("```"):
                code_lines.append(lines[index])
                index += 1
            code_text = "\n".join(code_lines).strip()
            if code_text == "导入 → 检查 → 修订 → 版本 → 期刊匹配 → 存证 → 投稿 → 知识体":
                story.extend([Spacer(1, 1 * mm), make_process_flow(styles), Spacer(1, 4 * mm)])
            else:
                story.append(Preformatted(code_text, styles["Code"], maxLineLength=92))
            index += 1
            continue
        if stripped.startswith("## "):
            heading_text = stripped[3:]
            heading_style = (
                styles["SectionHeadingCompact"]
                if heading_text == "3. 开始前准备"
                else styles["SectionHeading"]
            )
            story.append(Paragraph(inline_markup(heading_text), heading_style))
            index += 1
            continue
        if stripped.startswith("### "):
            story.append(Paragraph(inline_markup(stripped[4:]), styles["Subheading"]))
            index += 1
            continue
        if stripped.startswith("| "):
            table_lines = []
            while index < len(lines) and lines[index].strip().startswith("|"):
                table_lines.append(lines[index].strip())
                index += 1
            rows = []
            for table_line in table_lines:
                cells = [cell.strip() for cell in table_line.strip("|").split("|")]
                if all(re.fullmatch(r":?-{3,}:?", cell) for cell in cells):
                    continue
                rows.append(cells)
            if rows:
                story.extend([Spacer(1, 1.5 * mm), make_table(rows, styles), Spacer(1, 3 * mm)])
            continue
        if stripped.startswith("- "):
            items = []
            while index < len(lines) and lines[index].strip().startswith("- "):
                items.append(
                    ListItem(
                        Paragraph(inline_markup(lines[index].strip()[2:]), styles["Bullet"]),
                        leftIndent=0,
                    )
                )
                index += 1
            bullet_list = ListFlowable(
                items,
                bulletType="bullet",
                start="circle",
                leftIndent=4 * mm,
                bulletFontName="ManualSans",
                bulletFontSize=7,
                bulletColor=GREEN_DARK,
                spaceAfter=2 * mm,
            )
            story.append(bullet_list)
            continue
        numbered = re.match(r"^(\d+)\.\s+(.*)$", stripped)
        if numbered:
            items = []
            start_number = int(numbered.group(1))
            while index < len(lines):
                match = re.match(r"^(\d+)\.\s+(.*)$", lines[index].strip())
                if not match:
                    break
                items.append(
                    ListItem(
                        Paragraph(inline_markup(match.group(2)), styles["Numbered"]),
                        leftIndent=0,
                    )
                )
                index += 1
            story.append(
                ListFlowable(
                    items,
                    bulletType="1",
                    start=str(start_number),
                    leftIndent=5 * mm,
                    bulletFontName="ManualSans-Medium",
                    bulletFontSize=8.3,
                    bulletColor=GREEN_DARK,
                    spaceAfter=2 * mm,
                )
            )
            continue
        if stripped.startswith("> "):
            quote_lines = []
            while index < len(lines) and lines[index].strip().startswith(">"):
                quote_lines.append(lines[index].strip().lstrip("> "))
                index += 1
            story.append(Paragraph(inline_markup(" ".join(quote_lines)), styles["Callout"]))
            continue

        paragraph_lines = [stripped]
        index += 1
        while index < len(lines):
            candidate = lines[index].strip()
            if not candidate:
                break
            if candidate.startswith(("## ", "### ", "```", "- ", "|", ">")):
                break
            if re.match(r"^\d+\.\s+", candidate):
                break
            paragraph_lines.append(candidate)
            index += 1
        paragraph_text = " ".join(paragraph_lines)
        story.append(Paragraph(inline_markup(paragraph_text), styles["Body"]))
    return story


def main() -> None:
    register_fonts()
    styles = build_styles()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    doc = ManualDocTemplate(
        str(OUTPUT),
        pagesize=A4,
        leftMargin=LEFT,
        rightMargin=RIGHT,
        topMargin=TOP,
        bottomMargin=BOTTOM,
        title="投稿舱 ManuscriptDock 产品使用手册",
        author="ManuscriptDock",
        subject="Local-first manuscript submission workspace user manual",
        creator="ManuscriptDock documentation build",
    )
    doc.multiBuild(cover_story(styles) + toc_story(styles) + parse_markdown(styles))
    print(OUTPUT)


if __name__ == "__main__":
    main()
