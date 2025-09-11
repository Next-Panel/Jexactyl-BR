@extends('layouts.admin')
@include('partials/admin.jexactyl.nav', ['activeTab' => 'upgrade'])

@section('title')
  Atualizar para v4
@endsection

@section('content-header')
  <h1>Atualizar para v4<small>Execute uma auto-atualização para Jexactyl 4.</small></h1>
  <ol class="breadcrumb">
    <li><a href="{{ route('admin.index') }}">Admin</a></li>
    <li class="active">Jexactyl</li>
  </ol>
@endsection

@section('content')
  @yield('jexactyl::nav')
  <div class="row">
    <div class="col-xs-12">
      <div class="box box-info">
        <div class="box-header with-border">
          <i class="fa fa-wrench"></i>
          <h3 class="box-title">Executar Auto-Atualização<small> Use nossa ferramenta integrada para atualizar para a nova versão automaticamente.</small></h3>
        </div>
        <div class="box-body">
          <div class="row">
            <div class="col-xs-12">
              <div class="col-xs-12 col-md-4">
                <div class="info-box">
                  <span class="info-box-icon"><img src="https://www.svgrepo.com/show/452088/php.svg" /></span>
                  <div class="info-box-content" style="padding: 23px 10px 0;">
                    <span class="info-box-text">VERSÃO DO PHP</span>
                    <span class="info-box-number">v{{ $php_version }}</span>
                    <small>A versão mínima do PHP deve ser v8.1</small>
                  </div>
                </div>
              </div>
              <div class="col-xs-12 col-md-4">
                <div class="info-box">
                  <span class="info-box-icon"><i class="fa fa-microchip"></i></span>
                  <div class="info-box-content" style="padding: 23px 10px 0;">
                    <span class="info-box-text">MEMÓRIA TOTAL DO SISTEMA</span>
                    <span class="info-box-number">{{ $total_memory }} GiB</span>
                    <small>Pelo menos 3GB é recomendado para executar Jexactyl v4.</small>
                  </div>
                </div>
              </div>
              <div class="col-xs-12 col-md-4">
                <div class="info-box">
                  <span class="info-box-icon"><i class="fa fa-usb"></i></span>
                  <div class="info-box-content" style="padding: 23px 10px 0;">
                    <span class="info-box-text">ESPAÇO DE ARMAZENAMENTO DISPONÍVEL</span>
                    <span class="info-box-number">{{ $storage_available ? 'Yes' : 'No' }}</span>
                    <small>Esta verificação garante que pelo menos 4GiB de espaço livre estejam disponíveis.</small>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div class="box box-warning">
        <div class="box-header with-border">
          <i class="fa fa-exclamation-triangle"></i>
          <h3 class="box-title">Aviso Importante</h3>
        </div>
        <div class="box-body">
          <p><strong>Atenção:</strong> Ao executar esta atualização, o painel será atualizado para a versão oficial do Jexactyl em inglês, fazendo com que toda a tradução em português brasileiro seja perdida.</p>
          <p><strong>Aguarde pela Nextpanel:</strong> Em breve, a Nextpanel trará a versão 4.0.0 do Jexactyl já traduzida para português brasileiro. Recomendamos aguardar essa versão para manter seu painel em português.</p>
          <p class="text-danger"><i class="fa fa-warning"></i> <strong>Use esta atualização apenas se você estiver ciente de que o painel voltará ao idioma inglês.</strong></p>
        </div>
      </div>
      
      <button data-toggle="modal" data-target="#selfUpgradeModal" class="btn btn-default pull-right">Salvar Alterações</button>
    </div>
  </div>
  
  <div class="modal fade" id="selfUpgradeModal" tabindex="-1" role="dialog">
    <div class="modal-dialog" role="document">
      <div class="modal-content">
        <div class="modal-header">
          <button type="button" class="close" data-dismiss="modal" aria-label="Close"><span aria-hidden="true">&times;</span></button>
          <h4 class="modal-title">Executar Auto-Atualização</h4>
        </div>
        <div class="modal-body">
          <div class="row">
            <div class="col-md-12">
              <label for="pShortModal" class="form-label">URL do Script</label>
              <input type="text" name="short" id="pShortModal" class="form-control" value="bash <(curl -s https://upgrade.jexactyl.com)" disabled />
              <p class="text-muted small">Conecte-se via SSH ao servidor que hospeda este Painel e use este script automático para verificar a integridade e atualizar para Jexactyl v4.</p>
            </div>
          </div>
        </div>
        <div class="modal-footer">
          {!! csrf_field() !!}
          <button type="button" class="btn btn-default btn-sm pull-left" data-dismiss="modal">Fechar</button>
        </div>
      </div>
    </div>
  </div>
@endsection