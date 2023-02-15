<?php

namespace Pterodactyl\Notifications;

use Pterodactyl\Models\User;
use Illuminate\Bus\Queueable;
use Illuminate\Notifications\Notification;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Notifications\Messages\MailMessage;

class VerifyEmail extends Notification implements ShouldQueue
{
    use Queueable;

    public function __construct(private User $user, private string $name, private string $token)
    {
    }

    public function via(): array
    {
        return ['mail'];
    }

    public function toMail(): MailMessage
    {
        $message = new MailMessage();
        $message->greeting('Olá ' . $this->user->username . '! Bem vindo(a) ' . $this->name . '.');
        $message->line('Clique no link abaixo para verificar seu endereço de E-mail.');
        $message->action('Verificar E-mail', url('/auth/verify/' . $this->token));
        $message->line('Se você não criou esta conta, entre em contato ' . $this->name . '.');

        return $message;
    }
}
