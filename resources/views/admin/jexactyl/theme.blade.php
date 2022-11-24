@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'theme'])

@section('title')
    Configurações de tema
@endsection

@section('content-header')
    <h1>Aparência do Jexactyl<small>Configure o tema para Jexactyl.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li class="active">Jexactyl</li>
    </ol>
@endsection

@section('content')
    @yield('jexactyl::nav')
    <div class="row">
        <div class="col-xs-12">
            <form action="{{ route('admin.jexactyl.theme') }}" method="POST">
                <div class="box box-info
                ">
                    <div class="box-header with-border">
                        <h3 class="box-title">Temas para a área Admin <small>Escolha o tema do Jexactyl.</small></h3>
                    </div>
                    <div class="box-body">
                        <div class="row">
                            <div class="form-group col-md-4">
                                <label class="control-label">Tema Admin</label>
                                <div>
                                    <select name="theme:admin" class="form-control">
                                        <option @if ($admin == 'jexactyl') selected @endif value="jexactyl">Tema Padrão</option>
                                        <option @if ($admin == 'dark') selected @endif value="dark">Tema Escuro</option>
                                        <option @if ($admin == 'light') selected @endif value="light">Tema Claro</option>
                                        <option @if ($admin == 'blue') selected @endif value="blue">Tema Azul</option>
                                        <option @if ($admin == 'minecraft') selected @endif value="minecraft">Tema do Minecraft&#8482;</option>
                                    </select>
                                    <p class="text-muted"><small>Determina o tema para a IU Admin do Jexactyl.</small></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                {!! csrf_field() !!}
                <button type="submit" name="_method" value="PATCH" class="btn btn-default pull-right">Salvar alterações</button>
            </form>
        </div>
    </div>
@endsection
