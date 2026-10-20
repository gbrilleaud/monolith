import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

/// Configuration locale : aucun installeur n'est exécuté avant un clic explicite.
class EmulatorSettings extends StatefulWidget {
  const EmulatorSettings({
    super.key,
    required this.initialConfig,
    required this.onSave,
    required this.onInstall,
  });

  final String initialConfig;
  final Future<void> Function(String) onSave;
  final Future<String> Function(String) onInstall;

  @override
  State<EmulatorSettings> createState() => _EmulatorSettingsState();
}

class _EmulatorSettingsState extends State<EmulatorSettings> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialConfig,
  );
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.onSave(_controller.text);
      if (mounted) Navigator.of(context).pop(true);
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _install(String name) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final result = await widget.onInstall(name);
      if (mounted) setState(() => _error = result);
    } catch (error) {
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Émulateurs locaux'),
    content: SizedBox(
      width: 720,
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              'Pour Mega Drive (système 11), Monolith sélectionne automatiquement RetroArch portable et Genesis Plus GX s’ils sont présents dans tools/retroarch. Leur intégrité et le lancement réel restent à vérifier. '
              'Les autres consoles nécessitent encore un profil avancé. Les installateurs ci-dessous sont anciens et ne préparent pas un moteur géré complet.',
            ),
            const SizedBox(height: 14),
            TextField(
              controller: _controller,
              minLines: 9,
              maxLines: 16,
              decoration: const InputDecoration(
                labelText: 'emulators.toml',
                border: OutlineInputBorder(),
              ),
            ),
            if (defaultTargetPlatform == TargetPlatform.windows) ...[
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                children: [
                  for (final name in ['RetroArch', 'PCSX2', 'Xenia'])
                    OutlinedButton(
                      onPressed: _busy ? null : () => _install(name),
                      child: Text('Setup hérité $name'),
                    ),
                ],
              ),
            ],
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: SelectableText(
                  _error!,
                  style: const TextStyle(color: Color(0xFFFFC578)),
                ),
              ),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: _busy ? null : () => Navigator.of(context).pop(),
        child: const Text('Annuler'),
      ),
      FilledButton(
        onPressed: _busy ? null : _save,
        child: const Text('Enregistrer'),
      ),
    ],
  );
}
