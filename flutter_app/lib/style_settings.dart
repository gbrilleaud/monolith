import 'dart:convert';

import 'package:flutter/material.dart';

import 'style_theme.dart';

/// Editor presentation. Validation and writes always go through Rust/FRB.
class StyleSettings extends StatefulWidget {
  const StyleSettings({
    super.key,
    required this.snapshotJson,
    required this.onSelect,
    required this.onAnimation,
    required this.onRefresh,
    required this.onSaveCustom,
    this.onReset,
    this.onChanged,
  });
  final String snapshotJson;
  final Future<String> Function(String) onSelect;
  final Future<String> Function(bool) onAnimation;
  final Future<String> Function() onRefresh;
  final Future<String> Function(String) onSaveCustom;
  final Future<String> Function()? onReset;
  final ValueChanged<String>? onChanged;

  @override
  State<StyleSettings> createState() => _StyleSettingsState();
}

class _StyleSettingsState extends State<StyleSettings> {
  late StyleTheme _style = StyleTheme.fromSnapshot(widget.snapshotJson);
  late String _editorBase = _editorText(_style);
  late final TextEditingController _custom = TextEditingController(
    text: _editorBase,
  );
  bool _busy = false;
  String? _error;

  static String _editorText(StyleTheme style) {
    final template = Map<String, dynamic>.from(style.resolved);
    template['id'] = 'custom';
    template['name'] = 'Personnalisé';
    return const JsonEncoder.withIndent('  ').convert(template);
  }

  Future<void> _run(
    Future<String> Function() action, {
    bool edit = false,
  }) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final json = await action();
      final style = StyleTheme.fromSnapshot(json);
      widget.onChanged?.call(json);
      if (!mounted) return;
      setState(() {
        _style = style;
        if (!edit && _custom.text == _editorBase) {
          _editorBase = _editorText(style);
          _custom.text = _editorBase;
        }
      });
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  void dispose() {
    _custom.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 690),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Apparence de Monolith',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 12),
            const Text(
              'Choisissez un thème en un clic. Les presets du serveur '
              'peuvent être actualisés ; une copie locale reste disponible hors ligne.',
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final preset in _style.presets)
                  ChoiceChip(
                    label: Text(preset['name'] as String),
                    selected: _style.selected == preset['id'],
                    onSelected: _busy
                        ? null
                        : (_) => _run(
                            () => widget.onSelect(preset['id'] as String),
                          ),
                  ),
              ],
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: _busy
                  ? null
                  : () => _run(widget.onRefresh, edit: true),
              icon: const Icon(Icons.sync),
              label: const Text('Actualiser les thèmes du serveur'),
            ),
            SwitchListTile(
              title: const Text('Animations au lancement des jeux'),
              subtitle: const Text(
                'Toutes les consoles et le PC, version Flutter uniquement',
              ),
              value: _style.cartridgeAnimation,
              onChanged: _busy
                  ? null
                  : (enabled) =>
                        _run(() => widget.onAnimation(enabled), edit: true),
            ),
            const Divider(),
            const Text(
              'Thème personnalisé local (JSON) — couleurs #RRGGBB, '
              'police sans/serif/mono, densité, espacement, arrondis et '
              'image locale dans themes/backgrounds/.',
            ),
            const SizedBox(height: 8),
            TextField(
              key: const Key('style-custom-json'),
              controller: _custom,
              minLines: 8,
              maxLines: 13,
              decoration: const InputDecoration(
                labelText: 'style.json · preset custom',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            FilledButton(
              onPressed: _busy
                  ? null
                  : () => _run(
                      () => widget.onSaveCustom(_custom.text),
                      edit: true,
                    ),
              child: const Text('Enregistrer le thème personnalisé'),
            ),
            if (widget.onReset != null)
              TextButton.icon(
                onPressed: _busy ? null : () => _run(widget.onReset!),
                icon: const Icon(Icons.restore),
                label: const Text('Réinitialiser l’apparence locale'),
              ),
            if (_busy) const LinearProgressIndicator(),
            if (_error != null)
              Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
          ],
        ),
      ),
    ),
  );
}
