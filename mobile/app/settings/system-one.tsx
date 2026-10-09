// System One, the review model that answers how likely an action is safe. Auto-review alone
// uses it; no bot chats with it. Its key and model are the account's, so every paired Device
// shares them, encrypted in the credentials blob.

import { Stack, useRouter } from "expo-router";
import { useState } from "react";
import { ActivityIndicator, ScrollView, StyleSheet, Text } from "react-native";
import { engine } from "../../src/core/engine";
import { SYSTEM_ONE_BASE_URLS, SYSTEM_ONE_KIND } from "../../src/core/model";
import { useStore } from "../../src/core/store";
import { t, useLanguage } from "../../src/i18n";
import { alert } from "../../src/ui/alert";
import { FieldRow, Row, Section } from "../../src/ui/forms";
import { usePalette } from "../../src/ui/theme";

const DEFAULT_MODEL = "jev-latest";

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export default function SystemOneScreen() {
  useLanguage();
  const p = usePalette();
  const router = useRouter();
  const saved = useStore((s) => s.system_one);
  const [baseURL, setBaseURL] = useState(saved?.base_url ?? SYSTEM_ONE_BASE_URLS[0].url);
  const [model, setModel] = useState(saved?.model ?? DEFAULT_MODEL);
  const [apiKey, setAPIKey] = useState("");
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const root = baseURL.trim();
  const service = SYSTEM_ONE_BASE_URLS.find((known) => known.url === root);
  const canSave = !working && !!root && !!model.trim() && (!!apiKey.trim() || !!saved);

  async function save() {
    if (!canSave) return;
    setWorking(true);
    setError(null);
    try {
      const { warning } = await engine.connectSystemOne({ baseURL: root, apiKey, model });
      if (warning) alert(t("Saved with a warning"), warning, [{ text: t("OK"), onPress: () => router.back() }], { cancelable: false });
      else router.back();
    } catch (cause) {
      setError(messageOf(cause));
    } finally {
      setWorking(false);
    }
  }

  function confirmDisconnect() {
    alert(t("Disconnect System One?"), t("Auto-review stops using it. Its key is removed from every paired Device."), [
      { text: t("Cancel"), style: "cancel" },
      {
        text: t("Disconnect"),
        style: "destructive",
        onPress: () => {
          setWorking(true);
          setError(null);
          void engine
            .disconnectProvider(SYSTEM_ONE_KIND)
            .then(() => router.back())
            .catch((cause) => setError(messageOf(cause)))
            .finally(() => setWorking(false));
        },
      },
    ]);
  }

  return (
    <>
      <Stack.Screen options={{ title: "System One" }} />
      <ScrollView
        contentInsetAdjustmentBehavior="automatic"
        automaticallyAdjustKeyboardInsets
        contentContainerStyle={styles.content}
        keyboardDismissMode="on-drag"
        keyboardShouldPersistTaps="handled"
      >
        <Section footer={t("System One answers how likely an action is safe. It only reviews actions for Auto-review; no bot chats with it.")}>
          <Row
            title={t("Service")}
            menu={{
              title: t("Service"),
              value: service ? service.title : root || t("Custom"),
              choices: SYSTEM_ONE_BASE_URLS.map((known) => ({ title: known.title, selected: known.url === root, onPress: () => setBaseURL(known.url) })),
            }}
          />
          <FieldRow
            label={t("Base URL")}
            value={baseURL}
            onChangeText={setBaseURL}
            placeholder={SYSTEM_ONE_BASE_URLS[0].url}
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
            editable={!working}
            style={{ textAlign: "right", color: p.secondaryLabel }}
          />
        </Section>

        <Section footer={t("TypeSafe: jev-latest or jev-1.13.0. OpenRouter: typesafe/jev-1.13 or ~typesafe/jev-latest.")}>
          <FieldRow
            label={t("Model")}
            value={model}
            onChangeText={setModel}
            placeholder={DEFAULT_MODEL}
            autoCapitalize="none"
            autoCorrect={false}
            editable={!working}
            style={{ textAlign: "right", color: p.secondaryLabel }}
          />
        </Section>

        <Section footer={saved ? t("Left blank, the saved key is kept.") : undefined}>
          <FieldRow
            label={t("API Key")}
            value={apiKey}
            onChangeText={setAPIKey}
            placeholder={saved ? saved.detail : t("API key")}
            secureTextEntry
            autoCapitalize="none"
            autoCorrect={false}
            editable={!working}
            returnKeyType="done"
          />
        </Section>

        <Section>
          <Row title={saved ? t("Save") : t("Connect")} onPress={canSave ? () => void save() : undefined} accessory={working ? <ActivityIndicator /> : undefined} />
        </Section>

        {error ? <Text style={[styles.error, { color: p.red }]}>{error}</Text> : null}

        {saved ? (
          <Section>
            <Row title={t("Disconnect")} destructive onPress={!working ? confirmDisconnect : undefined} />
          </Section>
        ) : null}
      </ScrollView>
    </>
  );
}

const styles = StyleSheet.create({
  content: { paddingBottom: 40 },
  error: { marginHorizontal: 32, marginTop: 10, fontSize: 13, lineHeight: 18 },
});
