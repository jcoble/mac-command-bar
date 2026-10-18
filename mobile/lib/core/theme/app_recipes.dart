import 'package:flutter/material.dart';

import 'app_tokens.dart';

/// Reusable EdiPlatform design-system "recipes" (§7) ported to Flutter widgets.
///
///   * [M3TonalCard]    — §7.3 tinted tonal card (6 color families).
///   * [M3MorphNavItem] — §3.2 selection shape-morph (rounded-rect ↔ pill) for a
///     nav/segmented item, with a Material Symbols outline→fill cross-fade.

/// One of the six tonal-card / tinted-surface families (§2.5 / §7.3).
enum M3TonalFamily { mint, sky, amber, rose, violet, coral }

extension _TonalFamilyColors on M3TonalFamily {
  /// (background, accent) for the family, resolved off [tokens] (light pastels
  /// or dark faint-tints) and the [scheme] for accent ink.
  (Color bg, Color accent) resolve(AppTokens tokens, ColorScheme scheme) {
    switch (this) {
      case M3TonalFamily.mint:
        return (tokens.surfaceMint, tokens.success);
      case M3TonalFamily.sky:
        return (tokens.surfaceSky, tokens.info);
      case M3TonalFamily.amber:
        return (tokens.surfaceAmber, tokens.warning);
      case M3TonalFamily.rose:
        return (tokens.surfaceRose, scheme.error);
      case M3TonalFamily.violet:
        return (tokens.surfaceViolet, scheme.primary);
      case M3TonalFamily.coral:
        // Coral keeps its own warm family — tertiary is amber after the
        // TSK-162 accent pin and would duplicate the amber card.
        return (tokens.surfaceCoral, tokens.accentCoral);
    }
  }
}

/// §7.3 — A tinted tonal card. Background is a tinted surface (per [family]),
/// flat with a hairline tonal border. Optional [onTap] adds an ink state layer.
class M3TonalCard extends StatelessWidget {
  const M3TonalCard({
    super.key,
    required this.family,
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.onTap,
  });

  final M3TonalFamily family;
  final Widget child;
  final EdgeInsetsGeometry padding;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final tokens = context.tokens;
    final (bg, accent) = family.resolve(tokens, scheme);
    final border = accent.withValues(alpha: tokens.tonalBorderStrength * 0.5);

    return Material(
      color: bg,
      borderRadius: const BorderRadius.all(Radius.circular(28)),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Container(
          decoration: BoxDecoration(
            borderRadius: const BorderRadius.all(Radius.circular(28)),
            border: Border.all(color: border),
          ),
          padding: padding,
          child: child,
        ),
      ),
    );
  }
}

/// §3.2 / §7.5 — A nav/segmented item that morphs its corner shape on selection
/// (rounded-rect ↔ pill) and morphs its Material Symbol's **FILL axis** 0→1,
/// mirroring the web `--msym-fill` flip exactly (one variable glyph, not two).
///
/// Pass a `Symbols.*_rounded` glyph for [icon] (Material Symbols Rounded, the
/// font the web uses). The label is optional so this also works as an
/// icon-only bottom-nav destination.
class M3MorphNavItem extends StatelessWidget {
  const M3MorphNavItem({
    super.key,
    required this.icon,
    required this.selected,
    required this.onTap,
    this.label,
    this.iconSize = 24,
  });

  final IconData icon;
  final String? label;
  final bool selected;
  final double iconSize;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    // Violet active pill — matches the web sidebar's primary-container voice.
    final fg = selected ? scheme.onPrimaryContainer : scheme.onSurfaceVariant;

    return InkWell(
      onTap: onTap,
      borderRadius: const BorderRadius.all(Radius.circular(28)),
      child: AnimatedContainer(
        duration: M3Motion.medium3,
        curve: M3Motion.emphasizedDecelerate,
        padding: EdgeInsets.symmetric(
          horizontal: label == null ? 18 : 16,
          vertical: 8,
        ),
        decoration: BoxDecoration(
          color: selected ? scheme.primaryContainer : Colors.transparent,
          // Rest = pill (full); selected = rounded-rect (large). Mirrors the M3
          // Expressive selected-container shape morph.
          borderRadius: BorderRadius.circular(
            selected ? M3Shape.large : M3Shape.full,
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Animate the variable font's FILL axis 0→1 on selection (§7.5).
            TweenAnimationBuilder<double>(
              duration: M3Motion.medium2,
              curve: M3Motion.emphasizedDecelerate,
              tween: Tween(end: selected ? 1.0 : 0.0),
              builder: (context, fill, _) => Icon(
                icon,
                color: fg,
                size: iconSize,
                fill: fill,
                weight: selected ? 500 : 400,
              ),
            ),
            if (label != null) ...[
              const SizedBox(width: 8),
              Text(
                label!,
                style: theme.textTheme.labelLarge?.copyWith(
                  color: fg,
                  fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
