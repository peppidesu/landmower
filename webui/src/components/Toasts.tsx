import { useEffect, useId, useState } from "preact/hooks";

type ToastMessage = {
  type: "success" | "error" | "warning" | "info";
  message: string;
  duration?: number;
};

export default function Toasts() {
  const [toastMessages, setToastMessages] = useState<Map<string, ToastMessage>>(
    new Map(),
  );

  const addToast = (message: ToastMessage) => {
    const id = useId();
    setToastMessages(new Map([...toastMessages, [id, message]]));
    setTimeout(() => {
      setToastMessages(
        new Map([...toastMessages].filter(([key, _]) => key !== id)),
      );
    }, message.duration || 5000);
  };

  return (
    <div>
      {Array.from(toastMessages).map(([id, message]) => (
        <div key={id} class={`toast ${message.type}`}>
          {message.message}
        </div>
      ))}
    </div>
  );
}
