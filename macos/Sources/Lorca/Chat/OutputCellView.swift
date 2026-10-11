import AppKit

/// A published output in the transcript, standing without a bubble or a time: an image's
/// preview, the file's icon, name, type and size, and what the bot checked. A click opens the
/// output's sheet, or retries a file that could not be fetched; the menu opens or saves the
/// file, and replies to it. In a group the bot's avatar sits beside its bottom edge.
final class OutputCellView: TranscriptCellView {
    static let identifier = NSUserInterfaceItemIdentifier("OutputCell")

    static let width: CGFloat = 320

    /// The card's height at `rowWidth`, starting at `indent`. The table's row height and the
    /// cell's own layout come from the same `Layout`, so a card is exactly as tall as what it shows.
    static func height(for message: Message, rowWidth: CGFloat, indent: CGFloat) -> CGFloat {
        guard let output = message.output else { return 0 }
        return Layout(output: output, attachment: message.attachments.first, rowWidth: rowWidth, indent: indent).height
    }

    /// For a screen reader: the file, and how its check went.
    static func spokenText(output: Output) -> String {
        guard let evidence = output.evidence else { return output.name }
        return "\(output.name). \(evidence.title) · \(evidence.statusText): \(evidence.summary)"
    }

    /// "HTML · 18 KB", or the link's host.
    static func detail(output: Output, attachment: Attachment?) -> String {
        [output.typeName, attachment.map { Attachment.sizeText($0.size) } ?? ""]
            .filter { !$0.isEmpty }
            .joined(separator: " · ")
    }

    /// Where each part of a card sits, relative to the card.
    @MainActor private struct Layout {
        static let pad: CGFloat = 12
        static let iconSize: CGFloat = 26
        static let textX: CGFloat = pad + iconSize + 10
        static let headerHeight: CGFloat = 54
        static let checkIconSize: CGFloat = 15
        static let checkX: CGFloat = pad + checkIconSize + 8
        static let previewMax: CGFloat = 200
        static let previewMin: CGFloat = 96
        static let nameFont = NSFont.systemFont(ofSize: 13, weight: .semibold)
        static let detailFont = NSFont.systemFont(ofSize: 11.5)
        static let checkTitleFont = NSFont.systemFont(ofSize: 12, weight: .semibold)
        static let summaryFont = NSFont.systemFont(ofSize: 12)
        static let commandFont = NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)

        var width: CGFloat
        var height: CGFloat = 0
        var preview: NSRect?
        var headerY: CGFloat = 0
        var separatorY: CGFloat?
        var checkTitle = NSRect.zero
        var summary = NSRect.zero
        var command = NSRect.zero

        init(output: Output, attachment: Attachment?, rowWidth: CGFloat, indent: CGFloat) {
            width = min(OutputCellView.width, rowWidth - indent - ChatMetrics.horizontalInset)
            // An image keeps its box while its bytes are fetched or could not be, so the
            // transcript never moves.
            if output.kind == .image, let attachment {
                let full = Self.fullHeight(of: attachment, width: width)
                preview = NSRect(x: 0, y: 0, width: width, height: min(max(full, Self.previewMin), Self.previewMax))
            }
            headerY = preview?.maxY ?? 0
            var bottom = headerY + Self.headerHeight
            if let evidence = output.evidence {
                separatorY = bottom
                let textWidth = width - Self.checkX - Self.pad
                checkTitle = NSRect(x: Self.checkX, y: bottom + 9, width: textWidth, height: Self.measure("X", font: Self.checkTitleFont, width: textWidth))
                bottom = checkTitle.maxY
                let summaryText = evidence.summary.trimmingCharacters(in: .whitespacesAndNewlines)
                if !summaryText.isEmpty {
                    summary = NSRect(x: Self.checkX, y: bottom + 3, width: textWidth, height: Self.measure(summaryText, font: Self.summaryFont, width: textWidth))
                    bottom = summary.maxY
                }
                if let command = evidence.command, !command.isEmpty {
                    self.command = NSRect(x: Self.checkX, y: bottom + 3, width: textWidth, height: Self.measure("X", font: Self.commandFont, width: textWidth))
                    bottom = self.command.maxY
                }
                bottom += 10
            }
            height = bottom
        }

        /// The picture's height at the card's width.
        static func fullHeight(of attachment: Attachment, width: CGFloat) -> CGFloat {
            let pixels = attachment.width.flatMap { w in attachment.height.map { (w, $0) } } ?? (4, 3)
            return width * CGFloat(max(pixels.1, 1)) / CGFloat(max(pixels.0, 1))
        }

        static func measure(_ text: String, font: NSFont, width: CGFloat) -> CGFloat {
            TextMeasure.labelSize(of: NSAttributedString(string: text, attributes: [.font: font]), width: width).height
        }
    }

    private let avatar = AvatarView(diameter: ChatMetrics.avatarSize)
    private let card = BubbleView()
    private let preview = OutputPreviewView()
    private let previewRule = BackgroundView()
    private let outline = OutlineView()
    private let icon = NSImageView()
    private let name = Build.label("", font: Layout.nameFont)
    private let detail = Build.label("", font: Layout.detailFont, color: .secondaryLabelColor)
    private let separator = BackgroundView()
    private let checkIcon = NSImageView()
    private let checkTitle = Build.label("", font: Layout.checkTitleFont)
    private let summary = Build.label("", font: Layout.summaryFont, color: .secondaryLabelColor, lines: 0)
    private let command = Build.label("", font: Layout.commandFont, color: .tertiaryLabelColor)
    private var output: Output?
    private var attachment: Attachment?
    private var failed = false
    private var groupStart = true

    /// A click on the card: the output's sheet.
    var onOpen: (() -> Void)?
    /// A click on a card whose file could not be fetched.
    var onRetry: (() -> Void)?
    /// Open and Save As… (or Open Link and Copy Link), on a right-click.
    var fileMenu: (() -> NSMenu)?
    var onReply: (() -> Void)?
    var onFeedback: (() -> Void)?

    init() {
        super.init(frame: .zero)
        card.cornerRadius = 12
        card.fillColor = Theme.outputCard
        card.layer?.cornerRadius = 12
        card.layer?.masksToBounds = true
        // A tall picture draws past the box; a view no longer clips its drawing by default.
        preview.clipsToBounds = true
        previewRule.cornerRadius = 0
        previewRule.fillColor = .separatorColor
        separator.cornerRadius = 0
        separator.fillColor = .separatorColor
        icon.symbolConfiguration = NSImage.SymbolConfiguration(pointSize: 19, weight: .regular)
        checkIcon.symbolConfiguration = NSImage.SymbolConfiguration(pointSize: 13, weight: .medium)
        name.lineBreakMode = .byTruncatingMiddle
        detail.lineBreakMode = .byTruncatingTail
        summary.lineBreakMode = .byWordWrapping
        command.lineBreakMode = .byTruncatingTail
        for view in [preview, previewRule, icon, name, detail, separator, checkIcon, checkTitle, summary, command, outline] as [NSView] {
            card.addSubview(view.framePositioned())
        }
        addSubview(avatar.framePositioned())
        addSubview(card.framePositioned())
        card.addGestureRecognizer(NSClickGestureRecognizer(target: self, action: #selector(click)))
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError() }

    override var isFlipped: Bool { true }

    /// `avatar` is the bot's in a group, nil in a DM. `url` is the file's bytes on this Mac, nil
    /// until they are fetched; `error` is why they could not be.
    func configure(
        output: Output, attachment: Attachment?, url: URL?, error: String?, avatar avatarContent: AvatarView.Content?,
        groupStart: Bool
    ) {
        self.output = output
        self.attachment = attachment
        self.groupStart = groupStart
        failed = error != nil
        avatar.isHidden = avatarContent == nil
        if let avatarContent { avatar.content = avatarContent }
        icon.image = NSImage(systemSymbolName: output.symbolName, accessibilityDescription: nil)
        icon.contentTintColor = output.tint
        name.stringValue = output.name
        if let error {
            detail.stringValue = L("Couldn't download · Retry")
            detail.textColor = .systemRed
            card.toolTip = "\(output.name): \(error)"
        } else {
            detail.stringValue = Self.detail(output: output, attachment: attachment)
            detail.textColor = .secondaryLabelColor
            card.toolTip = url == nil && attachment != nil ? L("%@ · fetching…", output.name) : output.name
        }
        if output.kind == .image, let attachment, error == nil, let url {
            // Sharp at the card's widest, on a Retina screen: the longest side the picture
            // takes at that width, doubled.
            let full = Layout.fullHeight(of: attachment, width: Self.width)
            preview.image = Thumbnails.image(at: url, maxPixels: Int(min(4096, ceil(max(Self.width, full) * 2))))
        } else {
            preview.image = nil
        }
        if let evidence = output.evidence {
            let (symbol, tint): (String, NSColor) = switch evidence.status {
            case "passed": ("checkmark.circle.fill", .systemGreen)
            case "failed": ("xmark.circle.fill", .systemRed)
            default: ("circle.dashed", .secondaryLabelColor)
            }
            checkIcon.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
            checkIcon.contentTintColor = tint
            checkTitle.stringValue = "\(evidence.title) · \(evidence.statusText)"
            checkTitle.textColor = tint
            summary.stringValue = evidence.summary.trimmingCharacters(in: .whitespacesAndNewlines)
            command.stringValue = evidence.command ?? ""
            command.toolTip = evidence.command
        }
        needsLayout = true
    }

    @objc private func click() {
        failed ? onRetry?() : onOpen?()
    }

    override func menu(for event: NSEvent) -> NSMenu? {
        let menu = NSMenu()
        if failed {
            menu.addItem(ClosureMenuItem(L("Try Again")) { [weak self] in self?.onRetry?() })
        } else if let fileMenu {
            for item in fileMenu().items {
                item.menu?.removeItem(item)
                menu.addItem(item)
            }
        }
        if let onReply {
            if !menu.items.isEmpty { menu.addItem(.separator()) }
            menu.addItem(ClosureMenuItem(L("Reply"), handler: onReply))
            if let onFeedback { menu.addItem(ClosureMenuItem(L("Give Feedback…"), handler: onFeedback)) }
        }
        return menu.items.isEmpty ? nil : menu
    }

    func flash() {
        let pulse = CAKeyframeAnimation(keyPath: "opacity")
        pulse.values = [1, 0.35, 1, 0.35, 1]
        pulse.duration = 1.1
        card.layer?.add(pulse, forKey: "flash")
    }

    override func layout() {
        super.layout()
        guard let output else { return }
        let top = groupStart ? ChatMetrics.groupTopPadding : ChatMetrics.tightTopPadding
        let x = ChatMetrics.indent(showsAvatar: !avatar.isHidden)
        let layout = Layout(output: output, attachment: attachment, rowWidth: bounds.width, indent: x)

        card.frame = NSRect(x: x, y: top, width: layout.width, height: layout.height)
        outline.frame = card.bounds
        avatar.frame = NSRect(
            x: ChatMetrics.horizontalInset, y: top + layout.height - ChatMetrics.avatarSize,
            width: ChatMetrics.avatarSize, height: ChatMetrics.avatarSize)

        preview.isHidden = layout.preview == nil
        previewRule.isHidden = layout.preview == nil
        preview.frame = layout.preview ?? .zero
        previewRule.frame = NSRect(x: 0, y: layout.headerY - 1, width: layout.width, height: 1)

        let textWidth = layout.width - Layout.textX - Layout.pad
        icon.frame = NSRect(
            x: Layout.pad, y: layout.headerY + (Layout.headerHeight - Layout.iconSize) / 2,
            width: Layout.iconSize, height: Layout.iconSize)
        name.frame = NSRect(x: Layout.textX, y: layout.headerY + 10, width: textWidth, height: 17)
        detail.frame = NSRect(x: Layout.textX, y: layout.headerY + 28, width: textWidth, height: 15)

        let hasCheck = layout.separatorY != nil
        for view in [separator, checkIcon, checkTitle, summary, command] as [NSView] { view.isHidden = !hasCheck }
        guard let separatorY = layout.separatorY else { return }
        separator.frame = NSRect(x: 0, y: separatorY, width: layout.width, height: 1)
        checkIcon.frame = NSRect(
            x: Layout.pad, y: layout.checkTitle.midY - Layout.checkIconSize / 2,
            width: Layout.checkIconSize, height: Layout.checkIconSize)
        checkTitle.frame = layout.checkTitle
        summary.frame = layout.summary
        summary.isHidden = layout.summary.height == 0
        command.frame = layout.command
        command.isHidden = layout.command.height == 0
    }
}

/// An image output's picture across the top of its card: the whole picture when it is wide, the
/// top of it when it is tall, as a page's screenshot starts at its top. A quiet box until the
/// bytes are here.
private final class OutputPreviewView: NSView {
    var image: NSImage? { didSet { needsDisplay = true } }

    override var isFlipped: Bool { true }

    override func draw(_ dirtyRect: NSRect) {
        Theme.codeBackground.setFill()
        bounds.fill()
        guard let image, image.size.width > 0, image.size.height > 0 else { return }
        let full = bounds.width * image.size.height / image.size.width
        let rect: NSRect
        if full >= bounds.height {
            rect = NSRect(x: 0, y: 0, width: bounds.width, height: full)
        } else {
            let scale = max(bounds.width / image.size.width, bounds.height / image.size.height)
            let size = NSSize(width: image.size.width * scale, height: image.size.height * scale)
            rect = NSRect(x: (bounds.width - size.width) / 2, y: (bounds.height - size.height) / 2, width: size.width, height: size.height)
        }
        image.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: [.interpolation: NSImageInterpolation.high.rawValue])
    }
}

/// The card's hairline, drawn over the preview so the picture's top edge keeps the border.
private final class OutlineView: NSView {
    override var isFlipped: Bool { true }

    override func draw(_ dirtyRect: NSRect) {
        let path = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 11.5, yRadius: 11.5)
        NSColor.separatorColor.setStroke()
        path.lineWidth = 1
        path.stroke()
    }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}
